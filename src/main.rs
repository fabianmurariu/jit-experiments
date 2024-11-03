// Cargo.toml dependencies:
// [dependencies]
// mlua = { version = "0.9", features = ["luajit"] }

use mlua::{Lua, Result, Function};
use std::iter::Iterator;
use std::time::Instant;

struct Counter {
    current: u64,
    max: u64,
}

impl Counter {
    fn new(max: u64) -> Counter {
        Counter {
            current: 0,
            max,
        }
    }

    // Add a batch method to get multiple values at once
    fn next_batch(&mut self, batch_size: u64) -> Vec<u64> {
        let mut batch = Vec::with_capacity(batch_size as usize);
        for _ in 0..batch_size {
            if let Some(value) = self.next() {
                batch.push(value);
            } else {
                break;
            }
        }
        batch
    }
}

impl Iterator for Counter {
    type Item = u64;
    
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

fn sum_in_rust(max: u64) -> u64 {
    (0..max).sum()
}

fn main() -> Result<()> {
    let max_num = 5_000_000;
    let lua = Lua::new();
    
    // Time the pure Rust implementation
    let rust_start = Instant::now();
    let rust_sum = sum_in_rust(max_num);
    let rust_duration = rust_start.elapsed();
    println!("Rust sum: {} (took {:?})", rust_sum, rust_duration);

    // Set up the counter for Lua with batching
    let mut counter = Counter::new(max_num);
    let batch_size = 10000; // Adjust this value to find the sweet spot

    let next_batch = lua.create_function_mut(move |_, ()| {
        Ok(counter.next_batch(batch_size))
    })?;
    
    lua.globals().set("next_batch", next_batch)?;
    lua.globals().set("BATCH_SIZE", batch_size)?;
    
    // Run the Lua code with batching
    lua.load(r#"
        -- Enable JIT
        jit.opt.start(3)
        jit.opt.start("hotloop=1")
        jit.opt.start("sizemcode=512")  -- Increase the size of traces
        
        -- Print JIT status
        print(string.format("JIT Status: %s", jit.status() and "ENABLED" or "DISABLED"))
        print(string.format("LuaJIT version: %s", jit.version))
        
        -- Measure optimized hybrid implementation
        local start = os.clock()
        local sum = 0ULL
        
        while true do
            local batch = next_batch()
            if #batch == 0 then break end
            
            -- Sum the batch
            for _, value in ipairs(batch) do
                sum = sum + value
            end
        end
        
        hybrid_duration = os.clock() - start
        hybrid_sum = sum
        
        print(string.format("Optimized hybrid implementation took %.6f seconds (batch size: %d)", 
            hybrid_duration, BATCH_SIZE))
        
        -- Measure pure Lua implementation
        local function pure_lua_sum()
            local sum = 0ULL
            for i = 0, 5000000-1 do
                sum = sum + i
            end
            return sum
        end

        local pure_start = os.clock()
        local pure_lua_result = pure_lua_sum()
        pure_duration = os.clock() - pure_start

        print(string.format("Pure Lua sum: %d (took %.6f seconds)", 
            pure_lua_result, pure_duration))
    "#).exec()?;

    // Get results from global variables
    let lua_duration: f64 = lua.globals().get("hybrid_duration")?;
    let lua_sum: String = lua.globals().get("hybrid_sum")?;
    let lua_sum: u64 = lua_sum.parse().unwrap();

    println!("Optimized hybrid implementation sum: {} (took {:.6} seconds)", lua_sum, lua_duration);

    // Verify results match
    assert_eq!(rust_sum, lua_sum, "Rust and Lua sums should match!");

    Ok(())
}