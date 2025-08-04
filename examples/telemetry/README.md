# OpenTelemetry Integration Examples

This directory contains example OpenTelemetry data files for testing the `telemetry` command.

## Overview

The `telemetry` command enriches source code with runtime observability data from OpenTelemetry exports. It correlates traces, metrics, and logs with your source code using OpenTelemetry semantic conventions.

## Usage

### Basic Usage

```bash
# Analyze current directory with telemetry data
context-creator telemetry --telemetry-file traces.json

# Short form
context-creator telemetry -t traces.json
```

### With Specific Paths

```bash
# Analyze specific directories
context-creator telemetry -t traces.json src/ lib/

# Analyze a remote repository
context-creator telemetry -t traces.json --remote https://github.com/user/repo
```

### Filtering Options

```bash
# Filter by service name
context-creator telemetry -t traces.json --service payment-api

# Filter by time range (RFC3339 format)
context-creator telemetry -t traces.json --time-range "2024-01-01T00:00:00Z/2024-01-02T00:00:00Z"

# Combine filters
context-creator telemetry -t traces.json --service payment-api --time-range "2024-01-01T00:00:00Z/2024-01-02T00:00:00Z"
```

### Integration with Other Features

```bash
# Output to file
context-creator telemetry -t traces.json -o enriched-code.md

# Combine with semantic analysis
context-creator telemetry -t traces.json --trace-imports

# Use with LLM tools
context-creator telemetry -t traces.json --tool claude -p "Analyze the performance bottlenecks in this code"
```

## Example Files

### traces.json

A sample OTLP JSON export containing:
- Multiple spans from a payment processing service
- Code-related attributes (function name, file path, line number)
- Success and error scenarios
- Various latencies for percentile calculations

## OpenTelemetry Semantic Conventions

The telemetry command looks for these standard attributes:

- `code.function.name` - The name of the function being executed
- `code.file.path` or `code.filepath` - Path to the source file
- `code.line.number` or `code.lineno` - Line number in the source file
- `service.name` - Name of the service (for filtering)

## Output Format

The enriched output includes:

```markdown
<!-- OpenTelemetry Metrics
Calls: 1234 (last 24h)
Latency: p50=45ms, p95=120ms, p99=250ms
Error Rate: 0.3%
Most Common Error: "Payment gateway timeout"
-->
```

## Generating OTLP Data

To export telemetry data from your application:

1. **Using OpenTelemetry Collector**:
   ```yaml
   exporters:
     file:
       path: /path/to/traces.json
   ```

2. **Using OTLP HTTP Exporter**:
   ```bash
   curl http://localhost:4318/v1/traces > traces.json
   ```

3. **From Jaeger**:
   ```bash
   # Export traces from Jaeger (convert to OTLP format)
   curl http://localhost:16686/api/traces/{traceID} | jq '.data[0]' > trace.json
   ```

## Tips

1. **Large Files**: The telemetry command efficiently handles large OTLP files using streaming parsers
2. **Correlation**: Uses fuzzy matching for function names to handle namespaced functions
3. **Missing Data**: Gracefully handles spans without code attributes
4. **Performance**: Processes files in parallel when analyzing multiple paths