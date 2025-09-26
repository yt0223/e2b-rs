# E2B Rust SDK Troubleshooting

## Template Not Found Error

### Problem
```
Error: Api { status: 404, message: "{\"code\":404,\"message\":\"template 'nodejs' not found\"}" }
```

### Solution

The "nodejs" template doesn't exist in your E2B environment. You need to either:

1. **Use an existing template** (if you have any), or
2. **Create a new template** first

### Step 1: Check Available Templates

Run this command to see what templates you have:

```bash
cargo run --example list_templates
```

If you see templates listed, use one of those template IDs instead of "nodejs".

### Step 2: Create a New Template (if needed)

If you don't have any templates, create one:

```bash
cargo run --example create_nodejs_template
```

This will create a Node.js 18 template with common development tools.

### Step 3: Use the Correct Template ID

Once you have a template, update your code to use the correct template ID:

```rust
// Instead of:
let sandbox = client.sandbox().template("nodejs").create().await?;

// Use your actual template ID:
let sandbox = client.sandbox().template("your-actual-template-id").create().await?;
```

## Common Template Issues

### Issue: Build Failed
If your template build fails, check:
1. Dockerfile syntax is correct
2. Base image exists and is accessible
3. All RUN commands complete successfully
4. No network issues during package installation

### Issue: Template Takes Long to Build
Template builds can take several minutes depending on:
- Base image size
- Number of packages being installed
- Network speed
- Build complexity

Monitor progress in the [E2B Dashboard](https://e2b.dev/dashboard).

### Issue: Permission Denied
Make sure:
1. Your E2B_API_KEY is set correctly
2. Your account has permission to create templates
3. You haven't exceeded your template quota

## Quick Start Templates

Here are some common Dockerfile examples:

### Python 3.11
```dockerfile
FROM python:3.11-slim

RUN apt-get update && apt-get install -y \
    git \
    curl \
    vim \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
RUN pip install requests numpy pandas matplotlib

CMD ["python3", "--version"]
```

### Node.js 18
```dockerfile
FROM node:18-alpine

RUN apk add --no-cache git python3 make g++ bash curl vim

WORKDIR /app
RUN npm install -g typescript ts-node nodemon
RUN npm install express axios lodash

CMD ["node", "--version"]
```

### Ubuntu with Build Tools
```dockerfile
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    python3 \
    python3-pip \
    nodejs \
    npm \
    vim \
    nano \
    wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
CMD ["bash"]
```

## Getting Help

1. Check the [E2B Documentation](https://e2b.dev/docs)
2. Visit the [E2B Dashboard](https://e2b.dev/dashboard) to manage templates
3. Check build logs in the dashboard for detailed error messages
4. Ensure your API key is valid and has the necessary permissions

## API Key Setup

Make sure your API key is properly set:

```bash
export E2B_API_KEY=e2b_your_api_key_here
```

Or set it directly in code:
```rust
let client = Client::with_api_key("e2b_your_api_key_here");
```

Get your API key from: https://e2b.dev/dashboard?tab=keys