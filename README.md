# rs-cel-mcp: A CEL MCP Server

`rs-cel-mcp` is a server that implements the Model Context Protocol (MCP) to provide a Common Expression Language (CEL) evaluation tool to compatible clients, such as the Gemini CLI.

## How to Run

### 1. Build the Server

First, build the project in release mode:

```sh
cargo build --release
```

### 2. Run the Server

Once built, you can start the server using the `--http` flag to specify the listening address and port:

```sh
./target/release/cel-mcp --http 127.0.0.1:1234
```

The server will now be running and listening for requests on `http://127.0.0.1:1234`.

## How to Use with MCP Clients

You can connect this server to any MCP-compatible client. Here are instructions for two common clients.

### Gemini CLI

To allow the Gemini CLI to use this server, you need to add the server's configuration to your `settings.json` file.

To avoid overwriting your existing settings, you should merge the following configuration into the `mcpServers` object in your `settings.json`.

```json
{
  "mcpServers": {
    "cel-evaluator": {
      "httpUrl": "http://127.0.0.1:1234/mcp"
    }
  }
}
```

After adding the configuration and restarting the Gemini CLI, it will discover and be able to use the CEL evaluation tool.

### ollmcp

You can also use this server with [ollmcp](https://github.com/jonigl/mcp-client-for-ollama), a TUI client for Ollama.

While the `rs-cel-mcp` server is running, open a new terminal and run `ollmcp` with the `--mcp-server-url` flag pointing to the server's address.

```sh
ollmcp --mcp-server-url http://127.0.0.1:1234/mcp --model qwen2.5:7b
```

This will start `ollmcp` and connect it to your CEL server, allowing the specified Ollama model to use the `evaluate` tool.
