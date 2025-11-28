# SCRATCH

Temporary notes and ideas.

## JSON mapping to tasks

```json
{
  "project": {
    "id": 1,
    "name": "Build your own HTTP Server",
    "runner_image": "local|go|rust|c",
    "tasks": [
      {
        "id": 1,
        "name": "Bind to a Port",
        "description": "Create a TCP server that listens for incoming connections on port 4221.",
        "scores": "5:10:50|10:20:35|20:30:20",
        "status": "challenge_awaits",
        "abandoned_deduction": 5,
        "hints": [
          {
            "id": 1,
            "text": "Use the standard library's net package to create a TCP listener.",
            "unlock_criteria": "5:3:A",
            "points_deduction": 5
          }
        ],
        "validators": [
          "can_compile:bool(true)",
          "tcp_listening:int(4221)"
        ]
      },
      {
        "id": 2,
        "name": "Respond with 200 OK",
        "description": "Send a basic HTTP/1.1 200 OK response back to the client.",
        "scores": "5:10:50|10:20:35|20:30:20",
        "status": "challenge_awaits",
        "abandoned_deduction": 5,
        "hints": [
          {
            "id": 1,
            "text": "HTTP responses need \\r\\n line endings, not just \\n.",
            "unlock_criteria": "5:3:A",
            "points_deduction": 5
          },
          {
            "id": 2,
            "text": "Don't forget the blank line after headers (double \\r\\n).",
            "unlock_criteria": "10:6:A",
            "points_deduction": 10
          }
        ],
        "validators": [
          "http_response_status:int(200)"
        ]
      },
      {
        "id": 3,
        "name": "Extract the URL Path",
        "description": "Parse the HTTP request line to extract the URL path. Return 200 for / and 404 for other paths.",
        "scores": "8:15:75|15:25:50|25:40:30",
        "status": "challenge_awaits",
        "abandoned_deduction": 8,
        "hints": [
          {
            "id": 1,
            "text": "The request line format is: METHOD PATH HTTP/VERSION",
            "unlock_criteria": "5:3:A",
            "points_deduction": 8
          },
          {
            "id": 2,
            "text": "Split the first line by spaces to extract the path.",
            "unlock_criteria": "10:6:A",
            "points_deduction": 10
          }
        ],
        "validators": [
          "http_get:string(/),int(200)",
          "http_get:string(/unknown),int(404)",
          "http_get:string(/foo/bar),int(404)"
        ]
      },
      {
        "id": 4,
        "name": "Respond with Content",
        "description": "Implement /echo/{str} endpoint that returns {str} with Content-Type and Content-Length headers.",
        "scores": "10:20:100|20:35:70|35:50:40",
        "status": "challenge_awaits",
        "abandoned_deduction": 10,
        "hints": [
          {
            "id": 1,
            "text": "Content-Length must match the exact byte length of the body.",
            "unlock_criteria": "8:5:A",
            "points_deduction": 10
          },
          {
            "id": 2,
            "text": "Extract the string after /echo/ from the path.",
            "unlock_criteria": "12:8:A",
            "points_deduction": 15
          }
        ],
        "validators": [
          "http_get:string(/echo/hello),int(200),string(hello)",
          "http_get:string(/echo/abc),int(200),string(abc)",
          "http_header_present:string(Content-Type),bool(true)",
          "http_header_present:string(Content-Length),bool(true)"
        ]
      },
      {
        "id": 5,
        "name": "Parse Request Headers",
        "description": "Read HTTP headers and implement /user-agent endpoint returning the User-Agent header value.",
        "scores": "10:20:100|20:35:70|35:50:40",
        "status": "challenge_awaits",
        "abandoned_deduction": 10,
        "hints": [
          {
            "id": 1,
            "text": "Headers are formatted as 'Name: value' and are case-insensitive.",
            "unlock_criteria": "8:5:A",
            "points_deduction": 10
          },
          {
            "id": 2,
            "text": "Read lines until you hit an empty line (end of headers).",
            "unlock_criteria": "15:10:T",
            "points_deduction": 15
          }
        ],
        "validators": [
          "http_get_with_header:string(/user-agent),string(User-Agent),string(test-agent),int(200),string(test-agent)",
          "http_get_with_header:string(/user-agent),string(user-agent),string(curl/7.64.1),int(200),string(curl/7.64.1)"
        ]
      },
      {
        "id": 6,
        "name": "Handle Concurrent Connections",
        "description": "Modify your server to handle multiple simultaneous connections using threads or async I/O.",
        "scores": "15:30:125|30:50:85|50:70:50",
        "status": "challenge_awaits",
        "abandoned_deduction": 15,
        "hints": [
          {
            "id": 1,
            "text": "Spawn a new thread/goroutine for each accepted connection.",
            "unlock_criteria": "10:5:A",
            "points_deduction": 15
          },
          {
            "id": 2,
            "text": "Make sure the main loop continues accepting while handlers run.",
            "unlock_criteria": "20:10:T",
            "points_deduction": 20
          }
        ],
        "validators": [
          "concurrent_requests:int(3),string(/echo/test),int(200)"
        ]
      },
      {
        "id": 7,
        "name": "Serve Files from a Directory",
        "description": "Serve static files from a --directory flag. Return file contents or 404 if not found.",
        "scores": "15:30:125|30:50:85|50:70:50",
        "status": "challenge_awaits",
        "abandoned_deduction": 15,
        "hints": [
          {
            "id": 1,
            "text": "Use Content-Type: application/octet-stream for file responses.",
            "unlock_criteria": "10:5:A",
            "points_deduction": 12
          },
          {
            "id": 2,
            "text": "Watch out for path traversal attacks (../) - sanitize the path!",
            "unlock_criteria": "20:12:T",
            "points_deduction": 20
          }
        ],
        "validators": [
          "http_get_file:string(/files/test.txt),int(200)",
          "http_get_file:string(/files/nonexistent.txt),int(404)",
          "http_get_file:string(/files/../etc/passwd),int(404)"
        ]
      },
      {
        "id": 8,
        "name": "Handle POST Requests",
        "description": "Support POST requests to /files/{filename} to upload and save files.",
        "scores": "20:35:150|35:55:100|55:80:60",
        "status": "challenge_awaits",
        "abandoned_deduction": 18,
        "hints": [
          {
            "id": 1,
            "text": "Read exactly Content-Length bytes from the request body.",
            "unlock_criteria": "12:6:A",
            "points_deduction": 15
          },
          {
            "id": 2,
            "text": "Return 201 Created status code on successful file creation.",
            "unlock_criteria": "20:12:T",
            "points_deduction": 20
          }
        ],
        "validators": [
          "http_post_file:string(/files/upload.txt),string(hello world),int(201)",
          "file_contents_match:string(upload.txt),string(hello world),bool(true)"
        ]
      },
      {
        "id": 9,
        "name": "Support HTTP Compression",
        "description": "Add gzip compression support when client sends Accept-Encoding: gzip header.",
        "scores": "25:45:175|45:70:120|70:100:70",
        "status": "challenge_awaits",
        "abandoned_deduction": 20,
        "hints": [
          {
            "id": 1,
            "text": "Check if 'gzip' is in the Accept-Encoding header (may have multiple values).",
            "unlock_criteria": "15:8:A",
            "points_deduction": 18
          },
          {
            "id": 2,
            "text": "Use your language's gzip library. Don't forget Content-Encoding header.",
            "unlock_criteria": "25:15:T",
            "points_deduction": 25
          },
          {
            "id": 3,
            "text": "Content-Length must be the compressed size, not original size.",
            "unlock_criteria": "35:20:T",
            "points_deduction": 30
          }
        ],
        "validators": [
          "http_get_compressed:string(/echo/hello),string(gzip),int(200),string(hello)",
          "http_header_value:string(Content-Encoding),string(gzip),bool(true)",
          "http_get:string(/echo/hello),int(200),string(hello)"
        ]
      }
    ]
  }
}
```

## Field Documentation

### runner_image
Format: `"local|go|rust|c"`
- `local` = no runtime needed
- Uses `|` delimiter (not comma) to reserve commas for future tuple syntax (e.g., multi-service: web server + database)

### scores
Format: `"attempts:minutes:points|..."`
- Example: `"10:12:15|15:20:7"` means:
  - Bucket 1: 10 attempts OR 12 minutes → 15 points
  - Bucket 2: 15 attempts OR 20 minutes → 7 points
- Bucket selection: whichever threshold is hit first (`min(attempts, time)`)

### hints.unlock_criteria
Format: `"time:attempts:priority"`
- Example: `"10:30:T"` means unlock after 10 minutes OR 30 attempts, prioritize time (T)
- Priority: `T` = time-based, `A` = attempt-based

### status
Enum: `challenge_awaits | challenged | challenge_completed | challenge_failed | challenge_abandoned`

| Status | Description |
|--------|-------------|
| `challenge_awaits` | User has not started this task yet |
| `challenged` | User has started and is actively working |
| `challenge_completed` | User successfully finished the task |
| `challenge_failed` | User explicitly gave up or hit max attempts |
| `challenge_abandoned` | No activity for N time (set by backend job) |

State transitions:
```
challenge_awaits → challenged              (user starts task)
challenged → challenge_completed           (user passes all validators)
challenged → challenge_failed              (user gives up or max attempts)
challenged → challenge_abandoned           (no activity, backend job checks updated_at)
challenge_abandoned → challenged           (user returns and resumes)
```

### abandoned_deduction
Points deducted when user resumes an abandoned task.

Backend job runs every ~10 mins, checks `updated_at` column to determine if task should be marked `abandoned`.

### validators
Format: `"validator_name:param1,param2,...,paramN"`

**Type annotations:**
- `bool(true)` / `bool(false)` - boolean value
- `string(value)` - string value
- `int(123)` - integer value

**Examples:**
```
"can_compile:bool(true)"
"tcp_listening:int(4221)"
"http_get:string(/echo/hello),int(200),string(hello)"
"http_header_present:string(Content-Type),bool(true)"
```

**Notes:**
- Beginner projects: 1 validator per task
- Advanced projects: multiple validators per task (harder to earn points)
- Hints and points are based on "Task" level, not individual validations
