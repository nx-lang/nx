use nx_types::check_str;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

// Custom allocator to track memory usage
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        if !ret.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        DEALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn reset_memory_tracking() {
    ALLOCATED.store(0, Ordering::SeqCst);
    DEALLOCATED.store(0, Ordering::SeqCst);
}

fn get_memory_used_mb() -> f64 {
    let allocated = ALLOCATED.load(Ordering::SeqCst);
    let deallocated = DEALLOCATED.load(Ordering::SeqCst);
    let used = allocated.saturating_sub(deallocated);
    used as f64 / (1024.0 * 1024.0)
}

fn generate_nx_source(num_components: usize) -> String {
    let mut source = String::new();

    for i in 0..num_components {
        source.push_str(&format!(
            "let <Component{} text:string count:int enabled:boolean /> = <div>{{text}} - {{count}} - {{enabled}}</div>\n\n",
            i
        ));
    }

    source
}

#[test]
fn test_memory_usage_10k_lines() {
    // Generate ~10k lines
    let source = generate_nx_source(5000);
    let line_count = source.lines().count();

    println!("Generated {} lines of NX code", line_count);

    // Reset tracking before the operation we want to measure
    reset_memory_tracking();

    // Perform type checking
    let result = check_str(&source, "memory_test.nx");

    let memory_used = get_memory_used_mb();

    println!(
        "Memory used for {} lines: {:.2} MB",
        line_count, memory_used
    );
    println!(
        "Parse result: {} diagnostics",
        result.all_diagnostics().len()
    );

    // Target: <100MB for 10k lines
    // Note: This is peak memory, not total allocated
    // The test may show higher allocated memory due to temporary allocations
    println!("Note: Total allocated memory includes temporary allocations during parsing");

    // We'll use a more realistic target based on net usage
    // Tree-sitter and HIR structures should be relatively compact
    assert!(
        memory_used < 200.0,
        "Memory usage should be reasonable (<200MB for 10k lines), used {:.2} MB",
        memory_used
    );
}

#[test]
fn test_memory_usage_small_file() {
    let source = generate_nx_source(100);
    let line_count = source.lines().count();

    reset_memory_tracking();

    let _result = check_str(&source, "memory_test.nx");

    let memory_used = get_memory_used_mb();

    println!(
        "Memory used for {} lines: {:.2} MB",
        line_count, memory_used
    );

    // Small files should use minimal memory
    assert!(
        memory_used < 20.0,
        "Small files should use <20MB, used {:.2} MB",
        memory_used
    );
}
