import time

# Start time
start_time = time.time()

# Summing numbers from 0 to 50000000
total_sum = sum(range(0, 50000000))

# End time
end_time = time.time()

# Duration
duration = end_time - start_time

print(f"Sum: {total_sum}")
print(f"Time taken: {duration:.4f} seconds")

