# Databricks Provider Configuration

Jarvis supports Databricks as a model provider for LLM inference. This document describes how to configure Databricks as a provider.

## Overview

Databricks uses serving endpoints for model inference. These endpoints provide OpenAI-compatible APIs that can be used with Jarvis.

## Configuration

### Step 1: Get Your Databricks API Token

1. Log in to your Databricks workspace
2. Go to User Settings → Access Tokens
3. Generate a new token
4. Copy the token (you won't be able to see it again)

### Step 2: Set Environment Variable

Set the `DATABRICKS_API_KEY` environment variable with your token:

```bash
export DATABRICKS_API_KEY="your-token-here"
```

On Windows:
```powershell
$env:DATABRICKS_API_KEY="your-token-here"
```

### Step 3: Configure in config.toml

Add the Databricks provider to your `~/.jarvis/config.toml`:

```toml
[model_providers.databricks]
name = "Databricks"
base_url = "https://your-workspace.cloud.databricks.com/serving-endpoints/your-endpoint/invocations"
env_key = "DATABRICKS_API_KEY"
http_headers = { "Content-Type" = "application/json" }
```

Replace:
- `your-workspace` with your Databricks workspace name
- `your-endpoint` with your serving endpoint name

### Step 4: Use the Provider

Set the provider in your config or via command line:

```toml
model_provider = "databricks"
```

Or via command line:
```bash
jarvis --model-provider databricks "Your prompt here"
```

## URL Format

The base URL format for Databricks serving endpoints is:
```
https://{workspace}.cloud.databricks.com/serving-endpoints/{endpoint}/invocations
```

Where:
- `{workspace}` is your Databricks workspace name (e.g., `mycompany`)
- `{endpoint}` is your serving endpoint name (e.g., `gpt-4-endpoint`)

## Example Configuration

Here's a complete example configuration:

```toml
[model_providers.databricks]
name = "Databricks"
base_url = "https://mycompany.cloud.databricks.com/serving-endpoints/gpt-4-endpoint/invocations"
env_key = "DATABRICKS_API_KEY"
http_headers = { "Content-Type" = "application/json" }

model_provider = "databricks"
```

## Troubleshooting

### Authentication Errors

If you see authentication errors:
1. Verify your `DATABRICKS_API_KEY` is set correctly
2. Check that the token hasn't expired
3. Ensure the token has the necessary permissions

### Connection Errors

If you see connection errors:
1. Verify the base URL is correct
2. Check that the serving endpoint exists and is active
3. Ensure your network can reach the Databricks workspace

### Model Not Found

If you see model not found errors:
1. Verify the endpoint name in the URL
2. Check that the serving endpoint is deployed and active
3. Ensure the endpoint is accessible from your network

## Additional Resources

- [Databricks Serving Endpoints Documentation](https://docs.databricks.com/machine-learning/model-serving/index.html)
- [Databricks API Authentication](https://docs.databricks.com/dev-tools/auth/index.html)
