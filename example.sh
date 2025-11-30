#!/bin/bash

# ==============================================================================
# Configuration
# ==============================================================================
SERVER_ADDR="127.0.0.1:1234"
ENDPOINT="/mcp"
URL="http://${SERVER_ADDR}${ENDPOINT}"
ROOT_URL="http://${SERVER_ADDR}/"

# ==============================================================================
# Helper Functions
# ==============================================================================

# Checks if the server is running before proceeding.
check_server_is_running() {
	# We ping the root URL. We expect a 404, but any response indicates the server is up.
	# -sS hides progress but shows errors. A non-zero exit code will only occur on
	# connection errors, not on HTTP 4xx/5xx responses.
	if ! curl -sS --head "${ROOT_URL}" &>/dev/null; then
		echo "Error: Could not connect to the CEL MCP server at ${ROOT_URL}"
		echo "Please ensure the server is running in another terminal with the command:"
		echo ""
		echo "    RUST_LOG=debug target/release/cel-mcp --http ${SERVER_ADDR}"
		echo ""
		exit 1
	fi
	echo "Server is running. Proceeding with tests..."
	echo ""
}

# A helper function to send requests.
# It takes an ID and the JSON payload as arguments.
send_request() {
	local request_id="$1"
	local json_payload="$2"

	echo "Request (ID: ${request_id}):"
	echo "${json_payload}" | jq .
	echo ""
	echo "Response (ID: ${request_id}):"
	curl -s -X POST \
		-H "Content-Type: application/json" \
		-H "Accept: application/json, text/event-stream" \
		-d "${json_payload}" \
		"${URL}" | sed -e 's/^data: //g' | jq . # Strip 'data: ' prefix before piping to jq
	echo ""
}

# ==============================================================================
# Example Definitions
# ==============================================================================

run_ex0() {
	echo "--- [Example 0] Initializing MCP Session ---"

	local payload
	payload=$(jq -n \
		--argjson id 0 \
		--arg method "initialize" \
		--arg protocolVersion "2024-11-05" \
		--arg clientName "MCP-Client-Example" \
		--arg clientVersion "0.1.0" \
		'{ "jsonrpc": "2.0", "id": $id, "method": $method, "params": { "protocolVersion": $protocolVersion, "capabilities": {}, "clientInfo": { "name": $clientName, "version": $clientVersion } } }')

	send_request 0 "$payload"
}

run_ex1() {
	echo "--- [Example 1] Evaluate '1+2' (Mock) ---"

	local payload
	payload=$(jq -n \
		--argjson id 1 \
		--arg method "tools/call" \
		--arg tool_name "evaluate" \
		--arg expression "1+2" \
		'{ "jsonrpc": "2.0", "id": $id, "method": $method, "params": { "name": $tool_name, "arguments": { "expression": $expression, "context": {} } } }')

	send_request 1 "$payload"
}

run_ex2() {
	echo "--- [Example 2] Evaluate with context (a * b) ---"

	local payload
	payload=$(jq -n \
		--argjson id 2 \
		--arg method "tools/call" \
		--arg tool_name "evaluate" \
		--arg expression "a * b" \
		--argjson context '{"a": 5, "b": 10}' \
		'{ "jsonrpc": "2.0", "id": $id, "method": $method, "params": { "name": $tool_name, "arguments": { "expression": $expression, "context": $context } } }')

	send_request 2 "$payload"
}

run_ex3() {
	echo "--- [Example 3] Evaluate string concatenation ---"

	local payload
	payload=$(jq -n \
		--argjson id 3 \
		--arg method "tools/call" \
		--arg tool_name "evaluate" \
		--arg expression "'Hello, ' + name" \
		--argjson context '{"name": "World"}' \
		'{ "jsonrpc": "2.0", "id": $id, "method": $method, "params": { "name": $tool_name, "arguments": { "expression": $expression, "context": $context } } }')

	send_request 3 "$payload"
}

# ==============================================================================
# Main Execution Logic
# ==============================================================================

main() {
	echo "=================================================="
	echo "Testing CEL MCP server at ${URL}"
	echo "=================================================="
	echo ""

	check_server_is_running

	if [ -z "$1" ]; then
		# No arguments, run all examples
		run_ex0
		run_ex1
		run_ex2
		run_ex3
	else
		# Argument provided, run specific example
		case "$1" in
		0) run_ex0 ;;
		1) run_ex1 ;;
		2) run_ex2 ;;
		3) run_ex3 ;;
		*)
			echo "Error: Unknown example '$1'."
			echo "Usage: $0 [0|1|2|3]"
			exit 1
			;;
		esac
	fi

	echo ""
	echo "=================================================="
	echo "All tests complete."
	echo "=================================================="
}

main "$@"
