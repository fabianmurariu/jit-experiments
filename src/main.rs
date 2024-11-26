use wasmtime::*;
use std::time::Instant;

fn main() -> Result<()> {
    let max_num:i64 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "5000000".to_string())
        .parse()
        .expect("Invalid number");

    // Create engine and store
    let engine = Engine::new(&Default::default())?;

    // First implement our counter as WASM module using WAT format
    let counter_wat = wat::parse_str(
        r#"
        (module
            ;; Import memory from host
            (import "env" "memory" (memory 1))

            ;; Import host function to get next value
            (import "env" "next_value" (func $next_value (result i64)))

            ;; Export our sum function
            (func (export "sum") (result i64)
                (local $sum i64)
                (local $tmp i64)
                
                ;; Initialize sum to 0
                i64.const 0
                local.set $sum
                
                ;; Loop until we get None (represented as -1)
                (loop $counter_loop
                    ;; Get next value
                    call $next_value
                    local.set $tmp
                    
                    ;; Check if we got None (-1)
                    local.get $tmp
                    i64.const -1
                    i64.eq
                    (if
                        (then
                            local.get $sum
                            return
                        )
                    )
                    
                    ;; Add to sum
                    local.get $sum
                    local.get $tmp
                    i64.add
                    local.set $sum
                    
                    br $counter_loop
                )
                
                ;; Return final sum
                local.get $sum
            )
        )
    "#,
    )?;
    let counter_module = Module::new(&engine, counter_wat)?;

    // Create global counter state
    let counter_state = CounterState::new(max_num);
    let mut store = Store::new(&engine, counter_state);

    // Create memory
    let memory_type = MemoryType::new(1, None);
    let memory = Memory::new(&mut store, memory_type)?;

    // Define host function for next_value
    let next_value = Func::wrap(&mut store, |mut caller: Caller<'_, CounterState>| -> i64 {
        if let Some(value) = caller.data_mut().next() {
            value
        } else {
            -1
        }
    });

    let imports = [memory.into(), next_value.into()];

    // Instantiate module
    let instance = Instance::new(&mut store, &counter_module, &imports)?;

    // Get sum function
    let sum = instance
        .get_func(&mut store, "sum")
        .expect("sum function not found");

    // Time the WASM execution
    let wasm_start = Instant::now();
    let mut results = vec![Val::I64(0)];
    sum.call(&mut store, &[], &mut results)?;
    let wasm_duration = wasm_start.elapsed();

    println!("WASM sum: {:?} (took {:?})", &results[0], wasm_duration);

    // Compare with pure Rust implementation
    let rust_start = Instant::now();
    let rust_sum = CounterState::new(max_num).sum::<i64>();
    let rust_duration = rust_start.elapsed();

    println!("Rust sum: {} (took {:?})", rust_sum, rust_duration);

    assert_eq!(
        results[0].unwrap_i64(),
        rust_sum,
        "WASM and Rust sums should match!"
    );

    Ok(())
}

// Counter state to maintain between calls
struct CounterState {
    current: i64,
    max: i64,
}

impl CounterState {
    fn new(max: i64) -> Self {
        Self { current: 0, max }
    }
}

impl Iterator for CounterState {
    type Item = i64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.max {
            let result = self.current;
            self.current += 1;
            Some(result)
        } else {
            None
        }
    }
}
