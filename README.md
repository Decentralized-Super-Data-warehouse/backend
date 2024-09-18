# Decentralized Data Warehouse Backend

## Setup

1. **Start PostgreSQL Database:**

   ```sh
   docker compose up -d
   ```

   To clear the database, run:

   ```sh
   docker compose down -v
   ```

2. **Run the Backend:**

   ```sh
   cargo run
   ```

## API Documentation

Access Swagger UI at:

```
http://localhost:8080/swagger-ui/
```

## Notes

- Ensure Docker and Cargo are installed.
- Backend runs on port `8080`.
