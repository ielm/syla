# Syla Examples

This directory contains example payloads and test files for the Syla platform.

## API Execution Examples

Test the API Gateway with these example payloads:

```bash
# Python execution
curl -X POST http://localhost:8084/api/v1/executions \
  -H "Content-Type: application/json" \
  -d @examples/api-execution-python.json

# JavaScript execution  
curl -X POST http://localhost:8084/api/v1/executions \
  -H "Content-Type: application/json" \
  -d @examples/api-execution-javascript.json
```

## CLI Examples

```bash
# Execute a Python file
echo "print('Hello, Syla!')" > hello.py
syla exec hello.py

# Execute with explicit language
syla exec script.sh --language bash
```