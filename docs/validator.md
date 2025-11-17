## Validator needs

The tests only need to validate few things. 
1. `lux validate -t $task-id`  , imagine this is about building a http server. 
   1. $task-a checks if the server is running on port 8000
   2. $task-b checks if it has /api/v1/hello endpoint
   3. $task-c checks if it returns a json api
   4. $task-d checks if returns 404 status codes for endpoints that doesn’t exists
2. There will another type of validation, that are code validations. It will be strictly for go and rust. 
   1. So for data structure and algorithm courses, I’ll have lessons where users implement Doubly Linklist and Stacks and similar data structures. 
   2. So, I need a way `lux validate -t $task-id -f $file` that not only checks the output but is sees if the file has proper functions as well. Like `Push()` and `Pop()` etc.
3. Future implementations:
   1. Networking commands to check which ports are open
   2. Docker commands
   3. Kubernetes commands
   4. To explain these:
      1. I need to check if a container is healthy, so I’ll probably be running exec. So structure of my Task struct / trait must handle that, but also be generic enough.



