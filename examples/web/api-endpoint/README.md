# API Endpoint

A JSON API endpoint using `#content-type` and `#status` directives.

## Files

- `index.slt` — Returns a JSON response with user data

## Run

```bash
salata-server . --port 3000
curl http://localhost:3000/index.slt
```

## What It Demonstrates

- `#content-type application/json` sets the response MIME type
- `#status 200` sets the response status code
- Salata can serve API endpoints, not just HTML pages
