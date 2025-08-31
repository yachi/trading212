# Trading212 MCP Pagination Improvements Plan

## Problem
The Trading212 MCP server currently fetches ALL instruments from the API and applies client-side pagination, which is inefficient. MCP clients like Claude Code don't effectively use pagination parameters.

## Solution
Add clear pagination hints and improved response formatting to encourage proper pagination usage.

## Implementation Steps

### 1. Update Tool Description
- Enhance the `get_instruments` tool description to emphasize pagination
- Add performance recommendations

### 2. Improve Parameter Documentation
- Add detailed comments explaining pagination best practices
- Include examples of proper offset/limit usage

### 3. Enhanced Response Format
- Create new response helper that shows pagination metadata
- Include total count, current page info, and next page hints
- Add performance warnings for large datasets

### 4. Update GetInstrumentsTool
- Use the new paginated response format
- Provide clear guidance on next steps for pagination

## Expected Outcome
MCP clients will receive clear guidance on:
- When to use pagination
- How to properly paginate through large datasets
- Performance implications of large requests
- Next page parameters for continued iteration

This should encourage clients like Claude Code to automatically use appropriate pagination parameters instead of requesting all data at once.
