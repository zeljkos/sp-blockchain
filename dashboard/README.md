# SP Blockchain Dashboard

A minimalistic web interface for monitoring the 5-party Service Provider blockchain consortium.

## Features

- **Multi-tenant SP view**: Switch between different SP perspectives or global view
- **Real-time monitoring**: Live updates of blockchain, ZKP, and settlement data
- **Four main sections**:
  - Overview: System health and quick stats
  - Blockchain: Block information and chain statistics
  - ZKP & Smart Contracts: Zero-knowledge proof system status
  - Settlements: BCE records and multilateral netting data

## Access

The dashboard is available at: `http://localhost:8080/dashboard`

### Authentication

The dashboard requires SP authentication. Use one of the following API keys:

- T-Mobile DE: `tmobiledeapikey2024secure`
- Vodafone UK: `vodafoneuk_secure_api_2024`
- Orange FR: `orangefr_api_key_2024_secure`
- Telenor NO: `telenorno_2024_api_secure`
- SFR FR: `sfrfr_secure_2024_api_key`

### Access Methods

1. **Authorization Header**: `Authorization: Bearer <api_key>`
2. **API Key Header**: `X-API-Key: <api_key>`

## SP-Specific Views

Select your SP from the dropdown to see:
- Settlements where your SP is a participant
- BCE records filtered by your network
- ZKP proofs relevant to your settlements

## Global View

Select "Global View" to see:
- All consortium activity
- System-wide statistics
- Cross-SP settlement flows

## Data Refresh

- Click "Refresh" button to update all data
- Data auto-refreshes when switching tabs
- Real-time updates for active settlements

## Monitoring Capabilities

### System Health
- Blockchain node status
- ZKP system operational state
- Network connectivity health

### Blockchain Metrics
- Total blocks and chain height
- Settlement transaction count
- Recent block activity

### ZKP Performance
- Proof generation times
- Verification success rates
- Circuit performance metrics

### Settlement Analytics
- BCE record processing
- Multilateral netting efficiency (typically 75% reduction)
- Cross-SP payment flows

## Development

To run the dashboard in development:

```bash
# Build and start with dashboard
docker-compose -f docker/docker-compose-dashboard.yml up --build

# Access dashboard
open http://localhost:8080/dashboard
```

The dashboard is a single-page application that uses the existing SP blockchain API endpoints.