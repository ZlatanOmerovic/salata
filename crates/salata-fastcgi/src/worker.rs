// TODO: FastCGI worker — handles individual FastCGI requests.
//
// Responsibilities:
// - Parse the FastCGI record stream (BEGIN_REQUEST, PARAMS, STDIN, etc.)
// - Extract CGI environment variables and request body from FastCGI records
// - Run CGI protections (reuse salata_cgi::protection) on each request
// - Determine the .slt file to process from the CGI environment
// - Process .slt files through salata_core::process_file()
// - Reuse runtime processes across requests (persistent daemon advantage):
//   keep a pool of spawned interpreters (python, ruby, node, etc.) alive
//   between requests instead of spawning fresh processes each time
// - Encode the response as FastCGI STDOUT/STDERR/END_REQUEST records
// - Enforce execution time limits and memory caps per request
