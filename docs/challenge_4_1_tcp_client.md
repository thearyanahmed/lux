# Challenge 4.1: TCP Client - Test Scenarios

## Basic Connection Tests

1. **test_connect_success**
   - lux starts an echo server on random port
   - Client binary connects successfully
   - Verify: No connection errors, client doesn't crash

2. **test_connect_invalid_host**
   - Client tries to connect to invalid hostname
   - Verify: Graceful error message, non-zero exit code

3. **test_connect_refused**
   - Client tries to connect to port with no listener
   - Verify: Handles "connection refused" gracefully

4. **test_connect_timeout**
   - lux starts server that doesn't accept connections
   - Client should timeout after reasonable duration
   - Verify: Timeout error, not hanging forever

---

## Data Transmission Tests

5. **test_send_single_message**
   - Client sends "PING\n" to echo server
   - Verify: Server receives exactly "PING\n"

6. **test_receive_single_message**
   - Echo server sends "PONG\n"
   - Verify: Client prints "PONG" to stdout

7. **test_echo_roundtrip**
   - Client sends message, receives echo
   - Verify: Both send and receive work correctly

8. **test_multiple_messages**
   - Send 10 messages back and forth
   - Verify: All messages echoed correctly

9. **test_empty_message**
   - Client sends empty string or just "\n"
   - Verify: Handles gracefully without crashing

10. **test_large_message**
    - Send 10KB message
    - Verify: Entire message sent/received (not truncated)

11. **test_binary_data**
    - Send binary data (null bytes, special chars)
    - Verify: Data integrity maintained

---

## Protocol Tests

12. **test_line_delimited**
    - Send multiple lines: "line1\nline2\nline3\n"
    - Verify: Client processes line-by-line correctly

13. **test_partial_line**
    - Server sends data without newline
    - Verify: Client buffers and waits for complete line

14. **test_concurrent_read_write**
    - Client reads from server while sending
    - Verify: Full-duplex communication works

---

## Connection Lifecycle Tests

15. **test_graceful_close**
    - Send message, then close connection cleanly
    - Verify: No leaked file descriptors

16. **test_server_closes_first**
    - Server closes connection after 1 message
    - Verify: Client detects EOF, exits cleanly

17. **test_client_reconnect**
    - Connection drops mid-conversation
    - Verify: Client handles and potentially reconnects (if required)

18. **test_keepalive**
    - Keep connection open for 30 seconds with no data
    - Verify: Connection stays alive

---

## Error Handling Tests

19. **test_invalid_port**
    - Client given port 99999 (out of range)
    - Verify: Validation error before connection attempt

20. **test_stdin_closed**
    - Close stdin while client is running
    - Verify: Client detects EOF on stdin, closes gracefully

21. **test_network_interruption**
    - lux kills server mid-conversation
    - Verify: Client detects broken pipe/reset

22. **test_partial_write**
    - Server only reads half the message then stops
    - Verify: Client handles write blocking

---

## Performance Tests

23. **test_rapid_fire_messages**
    - Send 1000 small messages as fast as possible
    - Verify: All delivered, no data loss

24. **test_memory_leak**
    - Send 1000 messages, check process memory
    - Verify: No significant memory growth

---

## Edge Cases

25. **test_very_long_line**
    - Send single line of 1MB
    - Verify: Handles without buffer overflow

26. **test_no_trailing_newline**
    - Server sends data without final newline
    - Verify: Client handles EOF correctly

27. **test_mixed_newlines**
    - Send messages with \r\n, \n, \r
    - Verify: Consistent handling

28. **test_unicode_characters**
    - Send UTF-8 encoded emoji and multi-byte chars
    - Verify: Data integrity

---

## Integration Tests

29. **test_redis_ping**
    - Connect to real Redis server (if available)
    - Send "PING\r\n", expect "+PONG\r\n"
    - Verify: Works with real server protocol

30. **test_http_request**
    - Send basic HTTP GET request
    - Verify: Receives HTTP response

---

## Advanced Scenarios

31. **test_connection_pooling**
    - Open 10 concurrent connections
    - Verify: All work independently

32. **test_slow_server**
    - Server sends 1 byte per second
    - Verify: Client handles slow responses

33. **test_deadline_exceeded**
    - Set read deadline, server never responds
    - Verify: Client times out appropriately

---

## How lux Runs These Tests

```bash
# lux builds the user's code
$ lux verify 04-01

Running: go build -o client main.go
✓ Build successful

Test 1/33: test_connect_success
  ✓ Client connected to 127.0.0.1:54321
  ✓ No errors in stderr
  Duration: 45ms

Test 2/33: test_connect_refused
  ✓ Connection refused handled gracefully
  ✓ Exit code: 1
  Duration: 12ms

...

Results: 33/33 passed
```

Each test:
1. lux spawns a test server (in Rust)
2. lux executes `./client host port`
3. lux sends data via stdin or server socket
4. lux verifies stdout/stderr/exit code/network behavior
5. lux tears down and moves to next test
