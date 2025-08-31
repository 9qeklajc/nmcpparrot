# Nostr Memory MCP

A Model Context Protocol (MCP) server for storing and retrieving memories using the Nostr protocol with encryption.

## Features

- Store encrypted memories on Nostr relays
- Retrieve memories with filtering capabilities
- Multiple memory types (user_preference, context, fact, instruction, note)
- Categorization and tagging system
- Secure encryption for privacy

## Environment Variables

- `NOSTR_NSEC`: Your Nostr private key (nsec format)

## Usage

```bash
NOSTR_NSEC=your_private_key cargo run
```

## Tools

- `store_memory`: Store a new memory entry with encryption
- `retrieve_memories`: Retrieve memories with optional filtering
- `search_memories`: Search memories by content or tags

## Memory Types

- `user_preference`: User preferences and settings
- `context`: Contextual information
- `fact`: Factual information
- `instruction`: Instructions and how-tos  
- `note`: General notes and observations
