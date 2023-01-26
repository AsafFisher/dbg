# Basic Usage

Run the shellcode on any process and then perform the following to connect to it:
```python
import debugger

# Connect to the shellcode (it listens on 12345)
proc = debugger.RemoteProcess("10.0.0.2", 12345)
```

Now you have a `proc` object. This object "controlls" the process as if you run within it...
For example, you can write to an arbitrary address:
```python
# Get a memory address object for the address 0x4141414141414141 on the slave project
addr = proc.leak(0x4141414141414141)

# Write to that address "hello world"
amount_written = addr.write("hello world".encode("utf-8"))
```

You can read from that address:
```python
leaked_word = addr.read(5).decode("utf-8")
assert leaked_word == "hello"
```

You can call a function:
```python
fibo_func = proc.leak(ADDRESS_OF_FIBBO_FUNC)
result = fibo_func(44)
```

You can hook a function!
```python
def hook_strdup(original_hook, ptr, size):
    # Leak the original address
    x_addr = proc.leak(ptr)

    # Read the content of it
    x_content = x_addr.read(size)

    # If it is hello change it to bye
    if x_content == "hello":
        x_content = "bye"
    
    # Execute the original strdup function with the changed values
    return original_hook(x_content, len(x_content))

# Leak the addr of strdup
addr = proc.leak(ADDR_OF_STRDUP)

# Setup a hook for that function
hook = addr.hook(0xe, hook_strdup)

# Enable the hook.
hook.enable()

# Wait a little so the hook will be called a few times
sleep(100000)

# Disable the hook
hook.disable()
```