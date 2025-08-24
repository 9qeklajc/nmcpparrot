<div align="center">
  <img src="media/nparrot.png" alt="Logo" width="200" />
  <h1>nparrot</h1>
</div>

`nparrot` (_"Nostr Parrot"_) is a convenient CLI tool that facilitates one-on-one DM messaging over Nostr ([NIP-17](https://github.com/nostr-protocol/nips/blob/master/17.md)).

This tool has a few use cases and benefits:
- 🤖 Talk to your [Goose AI agent](https://block.github.io/goose/) via DMs from your phone by using any Nostr client compatible with [NIP-17](https://github.com/nostr-protocol/nips/blob/master/17.md).
- 🔗 Easily integrate almost any other command line tool with Nostr DMs.
- 🔔 Send yourself notifications easily from any environment that can run shell commands.
- 🧪 Use this as a test or troubleshooting tool for NIP-17 messages.

Here are the features it includes:
- ✅ **`send` command:** Sends a private NIP-17 direct message using specified arguments, or from stdin.
- ✅ **`wait` command:** Listens and waits for the next NIP-17 direct message from a specific user, and prints it to stdout once received.
- ✅ **`onmessage` command:** Continuously listens for NIP-17 direct messages, and for each one, it runs a shell command you specify.
- ✅ **`listen` command:** Continuously listens for NIP-17 direct messages, and for each one, it prints it to stdout.
- ✅ **`mcp` command:** MCP server that allows an AI agent to send a direct message to a specific user, or to wait for their message.
- 🆕 **`nostr-memory-mcp` command:** Advanced MCP server that provides persistent memory storage for AI agents using encrypted Nostr DMs.
- 🆕 **`combined-mcp` command:** Combined MCP server with chat, search, and command execution capabilities.
- 🆕 **`multi-agent-mcp` command:** Multi-agent MCP server for running multiple AI agents in parallel with integrated Nostr memory.

⚠️ **Note:** This is relatively new, experimental software. Please proceed with caution.

# Installation

We currently don't provide pre-built binaries. If you'd like a pre-built binary, please open an issue or contact us, we can help! Otherwise, please follow the build instructions below.

Make sure you have `cargo` 1.85 or higher installed. Then run:

```sh
cargo build --release
```

Then, you can find the executable binary on `./target/release/nparrot`, which you can run from there, or you can move it to another more convenient directory such as `~/.local/bin`.



# Talking to a goose AI agent via Nostr DMs

A very cool use case for this tool is the ability to talk to a [goose AI agent](https://block.github.io/goose/) on your phone, via Nostr DMs.

1. Download any Nostr client that supports NIP-17 DMs on your phone (e.g. [0xchat](https://www.0xchat.com)).
2. Go through onboarding.
3. Go to your profile, and find your npub (This will be in the form `npub1...`). Take note of that, this will be your `TARGET_PUBKEY`.
4. Also find and take note of the relays you are connected to. (e.g. `wss://relay.damus.io`). This will be your `RELAY_URL`.
5. Now, switch to your computer.
6. Generate a nostr private key (nsec) for your AI agent. If you have [`nak`](https://github.com/fiatjaf/nak), this can be done by simply running:
```
nak key generate | nak encode nsec
```
7. Take note of the `nsec` generated in the previous step, this will be your `NSEC`.
8. Now run these commands to set the variables on your environment (this helps make nparrot commands less verbose):
```
export TARGET_PUBKEY=<Your pubkey from step 3>
export NSEC=<Your nsec from step 6>
export RELAY_URL=<Your relay from step 4>
```
9. **Optional but recommended:** Send yourself a message to test if the setup works.
```
nparrot send "test"
```
9. **Optional but recommended:** Create a working directory where your AI agent will be working on, and `cd` to it.
10. **Optional but recommended:** Create a `.goosehints` on your working directory, with context to help the AI agent understand it should send you a message via Nostr DMs and not directly to the terminal. Example:
```markdown
- Talk to the user over nostr messaging using the tool provided.
- The user's name is Daniel
- Before you start doing any task, send a message to the user first saying indicating you are working on it
- You should always reply to the user over nostr, your normal output is not monitored.
```
9. Finally, run this command to start:
```
nparrot onmessage 'sed "s/^/New message from Nostr: /" | goose run --name="test-session" --with-extension "nparrot mcp" -i -'
```
10. Try sending a message from your app, and see your AI agent respond to it!

Notes:
- During testing, gpt-4o was used with good results

# AI Agent Memory with Nostr

The `nostr-memory-mcp` command provides a powerful memory system for AI agents using Nostr's encrypted direct messages. This allows agents to store and retrieve persistent memories across sessions.

## Features

🧠 **Memory Operations**: Store, retrieve, update, and delete memory entries  
🔐 **Privacy**: All memories encrypted using Nostr NIP-17  
📋 **Types**: Support for user preferences, context, facts, instructions, and notes  
📂 **Categories**: Organize memories by personal, work, project, or general categories  
🏷️ **Tags**: Tag-based organization and filtering  
📊 **Statistics**: Get insights about stored memories  
🧹 **Cleanup**: Automatic expiry and cleanup of old memories  

## Quick Start

```bash
# Set up environment
export NSEC=nsec1...
export TARGET_PUBKEY=npub1...

# Start the memory server
./nparrot nostr-memory-mcp
```

For detailed usage instructions, see [NOSTR_MEMORY_MCP_USAGE.md](NOSTR_MEMORY_MCP_USAGE.md).

# Enhanced Multi-Agent System

The `multi-agent-mcp` command now includes integrated Nostr memory capabilities, allowing agents to have persistent memory across sessions.

## Key Features

🤖 **Multi-Agent Orchestration**: Deploy and manage multiple AI agents in parallel  
🧠 **Persistent Memory**: Agents can store and recall information using Nostr  
🔄 **Local Fallback**: Memory system works even when Nostr is unavailable  
📊 **Agent Management**: Create, monitor, and coordinate multiple agent instances  
🎯 **Intelligent Routing**: Automatic agent selection based on task requirements  

## Quick Start

```bash
# Set up environment for memory functionality
export NSEC=nsec1...
export TARGET_PUBKEY=npub1...

# Start the enhanced multi-agent server
./nparrot multi-agent-mcp
```

Available agent types:
- **FuxScout** (search): Information gathering and research
- **FuxCoder** (goose): Development and engineering tasks  
- **FuxManager** (enhanced): Project coordination and management
- **FuxSpecialist** (combined): Multi-domain expertise
- **FuxComm** (chat): Communication coordination

For detailed information, see [MULTI_AGENT_NOSTR_MEMORY.md](MULTI_AGENT_NOSTR_MEMORY.md).

# Other commands

```
$ nparrot --help
Usage: nparrot [OPTIONS] --target-pubkey <TARGET_PUBKEY> --nsec <NSEC> <COMMAND>

Commands:
  send       Sends a private message via NIP-17. If the message is omitted, reads it from stdin
  wait       Waits for a private NIP-17 message to be received and prints the decrypted contents to stdout once received
  listen     Listens for private NIP-17 messages to be received and prints the decrypted contents to stdout after each one is received
  mcp        Starts an MCP server to allow an AI agent to manage the conversation
  onmessage  Runs a specified shell command each time it receives a NIP-17 direct message, passing the decrypted message contents to it via stdin
  help       Print this message or the help of the given subcommand(s)

Options:
      --target-pubkey <TARGET_PUBKEY>  Pubkey of the target user to talk to via DMs (in bech32 format) [env: TARGET_PUBKEY=]
      --nsec <NSEC>                    The private key (nsec) identity to use on the DMs [env: NSEC=]
      --relay <RELAY>                  Relay URL to use for sending/receiving messages [env: RELAY_URL=] [default: wss://relay.damus.io]
  -h, --help                           Print help
  -V, --version                        Print version
```

## Contributing

Contributions are welcome! Please open an issue or a pull request if you would like to contribute.

Please use `git commit --signoff` when committing changes to this repository, to certify that you agree to the [Developer Certificate of Origin](DCO.txt).
