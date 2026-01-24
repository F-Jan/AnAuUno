# Android Auto Protocol

## Android Auto Package

| Byte | Bit | Description           |
|------|-----|-----------------------|
| 1    | -   | Channel ID            |
| 2    | 0-1 | Frame Type            |
| 2    | 2   | 1: Is Control Message |
| 2    | 3   | 1: Encrypted          |
| 2    | 4-7 | Reserved              |
| 3-4  | -   | Length (Big Endian)   |

### Frame Types

| ID | Description |
|----|-------------|
| 0  | First       |
| 1  | None/Single |
| 2  | Middle      |
| 3  | Last        |

## Message Types

| ID | Name                       |
|----|----------------------------|
| 1  | Version-Request            |
| 2  | Version-Response           |
| 3  | Handshake                  |
| 4  | Handshake-OK               |
| 5  | Service-Discovery-Request  |
| 6  | Service-Discovery-Response |
