# Trading212 Public API Documentation

**Version:** v0
**Base URL:** `https://live.trading212.com/api/v0`
**Documentation:** https://t212public-api-docs.redoc.ly/

## Authentication

All API requests require authentication using an API key in the request header:

```
Authorization: {api_key}
```

### Required Scopes

Different endpoints require specific scopes:
- `pies:read` - Read investment pies
- `pies:write` - Create/modify investment pies
- `orders:execute` - Execute orders
- `account:read` - Read account data
- `portfolio:read` - Read portfolio data
- `history:read` - Read historical data

## Rate Limiting

Trading212 enforces strict rate limits **per account** (not per endpoint):

| Endpoint | Rate Limit | Notes |
|----------|------------|-------|
| `/equity/metadata/instruments` | **1 request per 50 seconds** | Strictest limit - use pagination |
| `/account` | 1 request per 30 seconds | Account information |
| `/account/cash` | 1 request per 2 seconds | Cash balance |
| `/equity/pies` | 1 request per 1 second | Shared limit with pie details |
| `/equity/pies/{pieId}` | 1 request per 1 second | Shared limit with pies list |
| `/equity/orders` | 1 request per 5 seconds | Orders |
| `/equity/portfolio` | 1 request per 5 seconds | Portfolio data |

**Important:**
- The MCP server includes **automatic rate limiting** that respects Trading212's `x-ratelimit-reset` headers
- **Pie endpoints share a rate limiter** because Trading212's limits apply per-account, not per-endpoint
- The server reads the `x-ratelimit-remaining` and `x-ratelimit-reset` response headers and waits until the exact reset time when limits are exhausted
- Base rate limiter: 2 seconds between requests (conservative buffer on top of Trading212's 1/1s limit)
- When `x-ratelimit-remaining=0`, the server automatically waits until `x-ratelimit-reset` timestamp before the next request
- This prevents 429 errors completely while maximizing throughput

## Endpoints

### Account Data

#### Get Account Cash
```
GET /equity/account/cash
```
Retrieve cash balance and available funds.

**Scope:** `account:read`
**Rate Limit:** 1/5s

**Response:**
```json
{
  "free": 1000.50,
  "total": 2500.00,
  "interest": 0.25,
  "commission": -5.00
}
```

#### Get Account Metadata
```
GET /equity/account/info
```
Retrieve general account information.

**Scope:** `account:read`
**Rate Limit:** 1/30s

### Instruments & Metadata

#### Get Exchanges
```
GET /equity/metadata/exchanges
```
Fetch list of supported exchanges.

**Scope:** None
**Rate Limit:** 1/30s

**Response:**
```json
[
  {
    "id": 123,
    "name": "NASDAQ",
    "workingSchedules": [...]
  }
]
```

#### Get Instruments
```
GET /equity/metadata/instruments
```
Retrieve list of available financial instruments.

**Scope:** None
**Rate Limit:** 1/30s

**Query Parameters:**
- `search` - Search by name or ticker
- `type` - Filter by instrument type

### Personal Portfolio

#### Get All Open Positions
```
GET /equity/portfolio
```
Retrieve all current open positions.

**Scope:** `portfolio:read`
**Rate Limit:** 1/1s

**Response:**
```json
[
  {
    "ticker": "AAPL",
    "quantity": 10,
    "averagePrice": 150.25,
    "currentPrice": 155.30,
    "marketValue": 1553.00,
    "unrealizedPnl": 50.50,
    "unrealizedPnlPct": 3.35
  }
]
```

#### Search Positions
```
GET /equity/portfolio/{ticker}
```
Get position details for a specific ticker.

**Scope:** `portfolio:read`
**Rate Limit:** 1/1s

**Path Parameters:**
- `ticker` - Stock ticker symbol

### Equity Orders

#### Place Market Order
```
POST /equity/orders/market
```
Execute a market order.

**Scope:** `orders:execute`
**Rate Limit:** 1/2s

**Request Body:**
```json
{
  "ticker": "AAPL",
  "quantity": 10,
  "timeValidity": "DAY"
}
```

#### Place Limit Order
```
POST /equity/orders/limit
```
Place a limit order.

**Scope:** `orders:execute`
**Rate Limit:** 1/2s

**Request Body:**
```json
{
  "ticker": "AAPL",
  "quantity": 10,
  "limitPrice": 150.00,
  "timeValidity": "DAY"
}
```

#### Place Stop Order
```
POST /equity/orders/stop
```
Place a stop order.

**Scope:** `orders:execute`
**Rate Limit:** 1/2s

**Request Body:**
```json
{
  "ticker": "AAPL",
  "quantity": 10,
  "stopPrice": 140.00,
  "timeValidity": "DAY"
}
```

#### Place Stop-Limit Order
```
POST /equity/orders/stop-limit
```
Place a stop-limit order.

**Scope:** `orders:execute`
**Rate Limit:** 1/2s

**Request Body:**
```json
{
  "ticker": "AAPL",
  "quantity": 10,
  "stopPrice": 140.00,
  "limitPrice": 138.00,
  "timeValidity": "DAY"
}
```

### Investment Pies

#### Get All Pies
```
GET /equity/pies
```
Retrieve all investment pies.

**Scope:** `pies:read`
**Rate Limit:** 1/1s

#### Get Pie Details
```
GET /equity/pies/{id}
```
Get detailed information about a specific pie.

**Scope:** `pies:read`
**Rate Limit:** 1/1s

**Path Parameters:**
- `id` - Pie ID

#### Create Pie
```
POST /equity/pies
```
Create a new investment pie.

**Scope:** `pies:write`
**Rate Limit:** 1/30s

**Request Body:**
```json
{
  "name": "Tech Stocks",
  "icon": "TECH",
  "instruments": [
    {
      "ticker": "AAPL",
      "weight": 50.0
    },
    {
      "ticker": "GOOGL",
      "weight": 30.0
    },
    {
      "ticker": "MSFT",
      "weight": 20.0
    }
  ]
}
```

#### Update Pie
```
POST /equity/pies/{id}
```
Update an existing pie allocation.

**Scope:** `pies:write`
**Rate Limit:** 1/30s

**Request Body:**
```json
{
  "instrumentShares": {
    "AAPL_US_EQ": 0.50,
    "GOOGL_US_EQ": 0.30,
    "MSFT_US_EQ": 0.20
  },
  "name": "Tech Stocks",
  "icon": "TECH",
  "goal": 10000.00,
  "dividendCashAction": "REINVEST",
  "endDate": "2029-12-31T23:59:59.999Z"
}
```

**Notes:**
- Uses POST method, not PATCH
- PATCH returns HTTP 405 Method Not Allowed
- `instrumentShares` is an object with ticker keys and decimal share values
- Shares should sum to 1.0 (100%)
- All fields are optional except the ones you want to update
- Legacy positions with 0% allocation remain but won't receive new investments

#### Delete Pie
```
DELETE /equity/pies/{id}
```
Delete a pie.

**Scope:** `pies:write`
**Rate Limit:** 1/30s

#### Duplicate Pie
```
POST /equity/pies/{id}/duplicate
```
Create a copy of an existing pie.

**Scope:** `pies:write`
**Rate Limit:** 1/30s

### Historical Data

#### Get Order History
```
GET /equity/history/orders
```
Retrieve historical order data.

**Scope:** `history:read`
**Rate Limit:** 1/6s

**Query Parameters:**
- `cursor` - Pagination cursor
- `limit` - Number of results (max 50)
- `ticker` - Filter by ticker

#### Get Dividend Data
```
GET /history/dividends
```
Retrieve dividend history.

**Scope:** `history:read`
**Rate Limit:** 1/6s

#### Get Transaction List
```
GET /history/transactions
```
Retrieve transaction history.

**Scope:** `history:read`
**Rate Limit:** 1/6s

#### Export Account Data
```
POST /equity/history/exports
```
Request account data export.

**Scope:** `history:read`
**Rate Limit:** 1/30s

**Request Body:**
```json
{
  "dataType": "TRANSACTIONS",
  "format": "CSV",
  "fromDate": "2024-01-01",
  "toDate": "2024-12-31"
}
```

## Common Parameters

### Time Validity Options
- `DAY` - Order valid for the trading day
- `GTC` - Good Till Cancelled

### Order Status Values
- `NEW` - Order placed but not executed
- `FILLED` - Order completely executed
- `PARTIALLY_FILLED` - Order partially executed
- `CANCELLED` - Order cancelled
- `REJECTED` - Order rejected

### Data Types for Exports
- `TRANSACTIONS`
- `ORDERS`
- `DIVIDENDS`

### Export Formats
- `CSV`
- `PDF`

## Error Handling

The API uses standard HTTP status codes:

- `200` - Success
- `400` - Bad Request
- `401` - Unauthorized (invalid API key)
- `403` - Forbidden (insufficient scope)
- `404` - Not Found
- `429` - Too Many Requests (rate limit exceeded)
- `500` - Internal Server Error

**Error Response Format:**
```json
{
  "error": "INSUFFICIENT_BALANCE",
  "message": "Not enough funds available",
  "details": {
    "required": 1000.00,
    "available": 500.00
  }
}
```

## Pagination

Many endpoints support pagination using cursor-based pagination:

```
GET /endpoint?cursor=abc123&limit=20
```

**Response includes:**
```json
{
  "items": [...],
  "nextPagePath": "/endpoint?cursor=def456&limit=20",
  "hasMore": true
}
```

## Best Practices

1. **Respect Rate Limits** - Monitor your request frequency
2. **Use Appropriate Scopes** - Request only necessary permissions
3. **Handle Errors Gracefully** - Implement proper error handling
4. **Cache Metadata** - Cache exchange and instrument data when possible
5. **Use Pagination** - Process large datasets in chunks
6. **Test in Demo Mode** - Use demo environment before production

## SDKs and Libraries

Consider using official or community SDKs for easier integration:
- Check Trading212's developer resources for official SDKs
- Community libraries may be available for popular languages

---

*This documentation is based on Trading212's Public API v0. For the most up-to-date information, always refer to the official documentation at https://t212public-api-docs.redoc.ly/*