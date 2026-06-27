use napi::bindgen_prelude::*;
use napi_derive::napi;
use nx_api::{
    build_workspace_program_artifact, diagnostics_to_api_with_source_entries,
    eval_program_artifact, load_program_artifact_from_source, validate_workspace, EvalResult,
    LibraryRegistry, NxDiagnostic, NxSeverity, NxWorkspace, NxWorkspaceModule, ProgramArtifact,
    ProgramBuildContext,
};
use nx_codegen::{emit_nx_ir, GeneratedNxIr, NxIrMetadata};
use nx_value::NxValue;
use serde::Serialize;

const EVALUATION_ERROR_PREFIX: &str = "NX_EVALUATION:";
const NATIVE_ERROR_PREFIX: &str = "NX_NATIVE:";
const DISPOSED_ERROR_PREFIX: &str = "NX_DISPOSED:";

#[derive(Clone, Copy)]
enum OutputFormat {
    MessagePack,
    Json,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedNxIrPayload {
    json: String,
    metadata: NxIrMetadata,
}

#[napi(object)]
pub struct NativeWorkspaceModule {
    pub identity: String,
    pub source: Either<String, Buffer>,
}

#[napi]
pub struct NativeNxWorkspace {
    workspace: Option<NxWorkspace>,
}

#[napi]
impl NativeNxWorkspace {
    #[napi(constructor)]
    pub fn new(modules: Vec<NativeWorkspaceModule>) -> Result<Self> {
        let mut workspace_modules = Vec::with_capacity(modules.len());
        for module in modules {
            let source = source_input_to_string(module.source)?;
            let workspace_module =
                NxWorkspaceModule::from_source(module.identity, source).map_err(input_error)?;
            workspace_modules.push(workspace_module);
        }

        let workspace = NxWorkspace::new(workspace_modules).map_err(input_error)?;
        Ok(Self {
            workspace: Some(workspace),
        })
    }

    #[napi]
    pub fn validate(&self, build_context: &NativeNxProgramBuildContext) -> Result<String> {
        let workspace = self.workspace()?;
        let build_context = build_context.build_context()?;
        let diagnostics = validate_workspace(workspace, build_context);
        diagnostics_json(&diagnostics)
    }

    #[napi]
    pub fn dispose(&mut self) {
        self.workspace = None;
    }

    fn workspace(&self) -> Result<&NxWorkspace> {
        self.workspace
            .as_ref()
            .ok_or_else(|| disposed_error("NxWorkspace"))
    }
}

#[napi]
pub struct NativeNxLibraryRegistry {
    registry: Option<LibraryRegistry>,
}

#[napi]
impl NativeNxLibraryRegistry {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            registry: Some(LibraryRegistry::new()),
        }
    }

    #[napi]
    pub fn load_library_from_directory(&self, root_path: String) -> Result<()> {
        let registry = self.registry()?;
        registry
            .load_library_from_directory(root_path)
            .map(|_| ())
            .map_err(evaluation_error)
    }

    #[napi]
    pub fn create_build_context(&self) -> Result<NativeNxProgramBuildContext> {
        let registry = self.registry()?;
        Ok(NativeNxProgramBuildContext {
            build_context: Some(registry.build_context()),
        })
    }

    #[napi]
    pub fn dispose(&mut self) {
        self.registry = None;
    }

    fn registry(&self) -> Result<&LibraryRegistry> {
        self.registry
            .as_ref()
            .ok_or_else(|| disposed_error("NxLibraryRegistry"))
    }
}

#[napi]
pub struct NativeNxProgramBuildContext {
    build_context: Option<ProgramBuildContext>,
}

#[napi]
impl NativeNxProgramBuildContext {
    #[napi]
    pub fn build_source_program_artifact(
        &self,
        source: Either<String, Buffer>,
        file_name: Option<String>,
    ) -> Result<NativeNxProgramArtifact> {
        let source = source_input_to_string(source)?;
        let file_name = normalize_file_name(file_name);
        let build_context = self.build_context()?;
        let program = load_program_artifact_from_source(&source, &file_name, build_context)
            .map_err(evaluation_error)?;
        Ok(NativeNxProgramArtifact::new(program))
    }

    #[napi]
    pub fn build_workspace_program_artifact(
        &self,
        workspace: &NativeNxWorkspace,
        entry_identity: String,
    ) -> Result<NativeNxProgramArtifact> {
        let build_context = self.build_context()?;
        let workspace = workspace.workspace()?;
        let program = build_workspace_program_artifact(workspace, &entry_identity, build_context)
            .map_err(evaluation_error)?;
        Ok(NativeNxProgramArtifact::new(program))
    }

    #[napi]
    pub fn dispose(&mut self) {
        self.build_context = None;
    }

    fn build_context(&self) -> Result<&ProgramBuildContext> {
        self.build_context
            .as_ref()
            .ok_or_else(|| disposed_error("NxProgramBuildContext"))
    }
}

#[napi]
pub struct NativeNxProgramArtifact {
    program: Option<ProgramArtifact>,
}

#[napi]
impl NativeNxProgramArtifact {
    fn new(program: ProgramArtifact) -> Self {
        Self {
            program: Some(program),
        }
    }

    #[napi(getter)]
    pub fn entry_identity(&self) -> Result<String> {
        Ok(self.program()?.entry_identity.clone())
    }

    #[napi]
    pub fn generate_nx_ir(&self) -> Result<String> {
        let program = self.program()?;
        let ir = emit_nx_ir(program).map_err(|error| codegen_error(error, program))?;
        generated_nx_ir_json(ir)
    }

    #[napi]
    pub fn evaluate_json(&self) -> Result<String> {
        match eval_program_artifact(self.program()?) {
            EvalResult::Ok(value) => value_json(&value),
            EvalResult::Err(diagnostics) => Err(evaluation_error(diagnostics)),
        }
    }

    #[napi]
    pub fn evaluate_bytes(&self, output_format: Option<String>) -> Result<Buffer> {
        let output_format = parse_output_format(output_format)?;
        match eval_program_artifact(self.program()?) {
            EvalResult::Ok(value) => value_bytes(&value, output_format).map(Buffer::from),
            EvalResult::Err(diagnostics) => Err(evaluation_error(diagnostics)),
        }
    }

    #[napi]
    pub fn dispose(&mut self) {
        self.program = None;
    }

    fn program(&self) -> Result<&ProgramArtifact> {
        self.program
            .as_ref()
            .ok_or_else(|| disposed_error("NxProgramArtifact"))
    }
}

fn normalize_file_name(file_name: Option<String>) -> String {
    match file_name {
        Some(file_name) if !file_name.is_empty() => file_name,
        _ => "input.nx".to_string(),
    }
}

fn source_input_to_string(source: Either<String, Buffer>) -> Result<String> {
    match source {
        Either::A(source) => Ok(source),
        Either::B(source) => String::from_utf8(source.to_vec()).map_err(|_| {
            input_error_message(
                "workspace-source-utf8",
                "NX source bytes must be valid UTF-8.",
            )
        }),
    }
}

fn parse_output_format(output_format: Option<String>) -> Result<OutputFormat> {
    match output_format.as_deref().unwrap_or("messagePack") {
        "messagePack" | "messagepack" | "msgpack" => Ok(OutputFormat::MessagePack),
        "json" => Ok(OutputFormat::Json),
        other => Err(input_error_message(
            "output-format",
            format!("Unsupported output format '{other}'. Use 'messagePack' or 'json'."),
        )),
    }
}

fn value_json(value: &NxValue) -> Result<String> {
    value
        .to_json_string()
        .map_err(|error| native_error(format!("Failed to serialize NX JSON value: {error}")))
}

fn value_bytes(value: &NxValue, output_format: OutputFormat) -> Result<Vec<u8>> {
    match output_format {
        OutputFormat::MessagePack => value.to_msgpack_vec().map_err(|error| {
            native_error(format!("Failed to serialize NX MessagePack value: {error}"))
        }),
        OutputFormat::Json => value_json(value).map(String::into_bytes),
    }
}

fn generated_nx_ir_json(ir: GeneratedNxIr) -> Result<String> {
    serde_json::to_string(&GeneratedNxIrPayload {
        json: ir.json,
        metadata: ir.metadata,
    })
    .map_err(|error| native_error(format!("Failed to serialize generated NX IR: {error}")))
}

fn diagnostics_json(diagnostics: &[NxDiagnostic]) -> Result<String> {
    serde_json::to_string(diagnostics)
        .map_err(|error| native_error(format!("Failed to serialize NX diagnostics: {error}")))
}

fn input_error(error: nx_api::NxWorkspaceInputError) -> Error {
    let diagnostic = NxDiagnostic {
        severity: NxSeverity::Error,
        code: Some("workspace-input-error".to_string()),
        message: error.to_string(),
        labels: Vec::new(),
        help: None,
        note: None,
    };
    evaluation_error(vec![diagnostic])
}

fn input_error_message(code: &'static str, message: impl Into<String>) -> Error {
    let diagnostic = NxDiagnostic {
        severity: NxSeverity::Error,
        code: Some(code.to_string()),
        message: message.into(),
        labels: Vec::new(),
        help: None,
        note: None,
    };
    evaluation_error(vec![diagnostic])
}

fn evaluation_error(diagnostics: Vec<NxDiagnostic>) -> Error {
    let payload = serde_json::to_string(&diagnostics).unwrap_or_else(|_| "[]".to_string());
    Error::new(
        Status::GenericFailure,
        format!("{EVALUATION_ERROR_PREFIX}{payload}"),
    )
}

fn codegen_error(error: nx_codegen::CodegenError, program: &ProgramArtifact) -> Error {
    let fallback_source = program
        .source_text(&program.entry_identity)
        .unwrap_or_default();
    let sources = program
        .source_entries()
        .into_iter()
        .map(|entry| (entry.identity, entry.source));
    let diagnostics =
        diagnostics_to_api_with_source_entries(&error.diagnostics, fallback_source, sources);
    evaluation_error(diagnostics)
}

fn native_error(message: impl Into<String>) -> Error {
    Error::new(
        Status::GenericFailure,
        format!("{NATIVE_ERROR_PREFIX}{}", message.into()),
    )
}

fn disposed_error(resource_name: &'static str) -> Error {
    Error::new(
        Status::GenericFailure,
        format!("{DISPOSED_ERROR_PREFIX}{resource_name}"),
    )
}
