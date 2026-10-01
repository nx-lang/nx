//! Linking: an entry module and the modules its table names, resolved into one program.

use crate::error::{Diagnostic, NxIrRuntimeError, Result};
use crate::module::{Declaration, ModuleData, PreparedModule, Ref, Shape};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// The reserved identity of the NX prelude, the module every NX module sees without an import.
pub const NX_PRELUDE_MODULE_IDENTITY: &str = "@nx/prelude.nx";

/// The compiled prelude this release was built with. `nx-codegen`'s tests keep it current.
static PRELUDE_IMAGE: &[u8] = include_bytes!("prelude.nxir");

/// The built-in prelude, prepared at most once per process and only when a link reaches it.
fn built_in_prelude() -> Result<PreparedModule> {
    static PRELUDE: OnceLock<Result<PreparedModule>> = OnceLock::new();
    PRELUDE
        .get_or_init(|| PreparedModule::prepare(PRELUDE_IMAGE))
        .clone()
}

/// How to link.
#[derive(Debug, Clone, Copy, Default)]
pub struct LinkOptions {
    /// Whether to link a module whose version differs from the one the entry recorded, as long
    /// as every declaration the entry references is present. Off by default.
    pub allow_version_mismatch: bool,
}

#[derive(Debug)]
pub(crate) struct Linked {
    pub module: Arc<ModuleData>,
    /// The program's module for each slot of this module's table; slot `0` is the module itself.
    pub slots: Vec<u32>,
}

#[derive(Debug)]
pub(crate) struct ProgramData {
    pub entry: Arc<ModuleData>,
    /// Module `0` is the entry.
    pub modules: Vec<Linked>,
    pub by_identity: HashMap<Arc<str>, u32>,
    /// The identity and image hash of every module, in identity order: what an instance records
    /// to say which program it belongs to.
    pub images: Arc<[(Arc<str>, u64)]>,
}

/// An entry module with every module it reaches, ready to evaluate.
///
/// <para>Cloning shares the program. A program holds its modules by reference, so linking many
/// snippets against one prepared catalog copies nothing.</para>
#[derive(Debug, Clone)]
pub struct Program {
    pub(crate) data: Arc<ProgramData>,
}

impl Program {
    /// Links `entry` against the modules its table names.
    ///
    /// <para>`resolve` is asked for each module by identity. The prelude is the one module no
    /// host has to supply: the built-in image answers for its identity whenever `resolve` does
    /// not, at any depth of the link. Linking fails naming the module when nothing supplies it,
    /// naming both versions when a resolved module's version is not the one recorded, and naming
    /// the declaration when a referenced declaration is absent.</para>
    pub fn link(
        entry: &PreparedModule,
        mut resolve: impl FnMut(&str) -> Option<PreparedModule>,
        options: &LinkOptions,
    ) -> Result<Self> {
        let mut diagnostics = Vec::new();
        let mut modules: Vec<Linked> = Vec::new();
        let mut by_identity: HashMap<Arc<str>, u32> = HashMap::new();
        let add = |module: &PreparedModule,
                   modules: &mut Vec<Linked>,
                   by_identity: &mut HashMap<Arc<str>, u32>| {
            let index = modules.len() as u32;
            by_identity.insert(Arc::clone(&module.data.identity), index);
            modules.push(Linked {
                module: Arc::clone(&module.data),
                slots: vec![index],
            });
            index
        };
        add(entry, &mut modules, &mut by_identity);

        // Modules are appended as they are met and linked in that order, so a module that names
        // itself through another module's table links to one entry rather than recursing.
        let mut next = 0;
        while let Some(module) = modules.get(next).map(|linked| Arc::clone(&linked.module)) {
            let own = next as u32;
            for (slot, wanted) in module.table.iter().enumerate().skip(1) {
                let resolved = match by_identity.get(&wanted.identity) {
                    Some(index) => modules.get(*index as usize).map(|linked| PreparedModule {
                        data: Arc::clone(&linked.module),
                    }),
                    None => resolve(&wanted.identity),
                };
                let resolved = match resolved {
                    Some(resolved) => Some(resolved),
                    None if &*wanted.identity == NX_PRELUDE_MODULE_IDENTITY => {
                        match built_in_prelude() {
                            Ok(prelude) => Some(prelude),
                            Err(error) => {
                                diagnostics.push(Diagnostic::new(
                                    "nx-ir-prelude-image",
                                    format!("The built-in prelude could not be prepared: {error}"),
                                ));
                                None
                            }
                        }
                    }
                    None => None,
                };
                let Some(resolved) = resolved else {
                    diagnostics.push(Diagnostic::new(
                        "nx-ir-link-missing-module",
                        format!(
                            "Module '{}' links against '{}', which the resolver did not supply.",
                            module.identity, wanted.identity
                        ),
                    ));
                    if let Some(linked) = modules.get_mut(next) {
                        linked.slots.push(own);
                    }
                    continue;
                };
                if resolved.data.identity != wanted.identity {
                    diagnostics.push(Diagnostic::new(
                        "nx-ir-link-identity",
                        format!(
                            "The resolver answered '{}' with a module whose identity is '{}'.",
                            wanted.identity, resolved.data.identity
                        ),
                    ));
                }
                if resolved.data.version != wanted.version && !options.allow_version_mismatch {
                    diagnostics.push(Diagnostic::new(
                        "nx-ir-link-version",
                        format!(
                            "Module '{}' was compiled against '{}' version '{}', but the resolved module is version '{}'.",
                            module.identity, wanted.identity, wanted.version, resolved.data.version
                        ),
                    ));
                }
                for name in module.external_references.get(slot).into_iter().flatten() {
                    if !resolved.data.by_name.contains_key(name) {
                        diagnostics.push(Diagnostic::new(
                            "nx-ir-link-missing-declaration",
                            format!(
                                "Module '{}' references '{name}' in '{}', which does not declare it.",
                                module.identity, wanted.identity
                            ),
                        ));
                    }
                }
                let index = match by_identity.get(&resolved.data.identity) {
                    Some(index) => *index,
                    None => add(&resolved, &mut modules, &mut by_identity),
                };
                if let Some(linked) = modules.get_mut(next) {
                    linked.slots.push(index);
                }
            }
            next = next.saturating_add(1);
        }

        if !diagnostics.is_empty() {
            return Err(NxIrRuntimeError { diagnostics });
        }
        let mut images: Vec<(Arc<str>, u64)> = modules
            .iter()
            .map(|linked| {
                (
                    Arc::clone(&linked.module.identity),
                    linked.module.image_hash,
                )
            })
            .collect();
        images.sort();
        Ok(Self {
            data: Arc::new(ProgramData {
                entry: Arc::clone(&entry.data),
                modules,
                by_identity,
                images: Arc::from(images),
            }),
        })
    }

    /// Prepares and links a self-contained image: one whose module table names only itself, or
    /// only itself and the prelude.
    pub fn prepare(bytes: impl Into<Arc<[u8]>>) -> Result<Self> {
        Self::link(
            &PreparedModule::prepare(bytes)?,
            |_| None,
            &LinkOptions::default(),
        )
    }

    /// The entry module.
    pub fn entry(&self) -> PreparedModule {
        PreparedModule {
            data: Arc::clone(&self.data.entry),
        }
    }

    /// The identities of the program's modules, the entry first.
    pub fn modules(&self) -> impl Iterator<Item = &str> {
        self.data
            .modules
            .iter()
            .map(|linked| &*linked.module.identity)
    }
}

impl PreparedModule {
    /// This module as a program, when its table names no other module.
    ///
    /// <para>A module that names other modules must be linked with [`Program::link`], and this
    /// fails with `nx-ir-unlinked` rather than evaluating it against nothing.</para>
    pub fn program(&self) -> Result<Program> {
        if self.data.table.len() > 1 {
            return crate::error::fail(
                "nx-ir-unlinked",
                format!(
                    "Module '{}' names {} other module(s) in its table and must be linked with Program::link before it is evaluated.",
                    self.data.identity,
                    self.data.table.len().saturating_sub(1)
                ),
            );
        }
        Program::link(self, |_| None, &LinkOptions::default())
    }
}

impl ProgramData {
    pub(crate) fn linked(&self, module: u32) -> Option<&Linked> {
        self.modules.get(module as usize)
    }

    /// The program's module that `slot` of `module`'s table names.
    pub(crate) fn target(&self, module: u32, slot: u32) -> Option<u32> {
        self.linked(module)?.slots.get(slot as usize).copied()
    }

    /// The identity of the program's module `module`.
    pub(crate) fn identity(&self, module: u32) -> Arc<str> {
        self.linked(module)
            .map(|linked| Arc::clone(&linked.module.identity))
            .unwrap_or_else(|| Arc::from(""))
    }

    /// The key that identifies one declaration across the program: its module's identity and
    /// its name.
    pub(crate) fn key(&self, module: u32, reference: &Ref) -> String {
        let target = self.target(module, reference.slot).unwrap_or(module);
        format!("{}::{}", self.identity(target), reference.name)
    }

    /// The declaration `reference` names from `module`, with the module that declares it.
    pub(crate) fn resolve(
        &self,
        module: u32,
        reference: &Ref,
    ) -> std::result::Result<(u32, u32, &Declaration), String> {
        let identity = |module: u32| self.identity(module);
        let target = self.target(module, reference.slot).ok_or_else(|| {
            format!(
                "Module '{}' has no module at slot {}.",
                identity(module),
                reference.slot
            )
        })?;
        let (index, declaration) = self
            .linked(target)
            .and_then(|linked| linked.module.find(&reference.name))
            .ok_or_else(|| {
                format!(
                    "Module '{}' does not declare '{}'.",
                    identity(target),
                    reference.name
                )
            })?;
        Ok((target, index, declaration))
    }

    /// The constructible shapes of every linked module that a value's `$type` names. More than
    /// one when two modules each declare a record of one name; the caller decides what to do
    /// about that rather than being handed a guess.
    pub(crate) fn shapes<'a, 'd>(
        &'a self,
        discriminator: &'d str,
    ) -> impl Iterator<Item = (u32, &'a Shape)> + use<'a, 'd> {
        self.modules
            .iter()
            .enumerate()
            .flat_map(move |(index, linked)| {
                linked
                    .module
                    .shapes
                    .get(discriminator)
                    .into_iter()
                    .flatten()
                    .map(move |shape| (index as u32, shape))
            })
    }
}
