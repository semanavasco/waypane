# Shell Commands

`waypane` provides helper functions to execute shell commands and poll for their output. These are useful for integrating external scripts or system information into your desktop environment.

## `waypane.exec(cmd, callback)`

Executes a shell command asynchronously.

- **`cmd`**: The shell command string to execute.
- **`callback`**: (Optional) A function that receives `(stdout, stderr)` as arguments when the command finishes.

### Usage Example

```lua
-- Execute a command without waiting for its output
waypane.exec("pkill hyprpaper")

-- Execute a command and handle its output
waypane.exec("uptime -p", function(stdout, stderr)
    if stdout ~= "" then
        print("Uptime: " .. stdout)
    end
end)
```

## `waypane.poll(cmd, callback, interval)`

Executes a shell command repeatedly at a given interval and calls a callback with its output.

- **`cmd`**: The shell command string to poll.
- **`callback`**: A function that receives `(stdout, stderr)` as arguments after each command execution.
- **`interval`**: The time in milliseconds to wait between executions.
- **Returns**: A [`CancelHandle`](./timers.md#cancelhandle) that can be used to stop the polling.

### Usage Example

```lua
-- Poll for memory usage every 5 seconds
local mem_poll = waypane.poll("free -h | awk '/^Mem:/ {print $3 \" / \" $2}'", function(stdout, stderr)
    print("Memory Usage: " .. stdout)
end, 5000)

-- Later, stop polling:
mem_poll:cancel()
```
