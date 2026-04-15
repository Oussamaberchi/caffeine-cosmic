# Contributing to Caffeine for COSMIC

Contributions are welcome! This project is maintained by Oussama Berchi and mmstick.

## How to Contribute

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run tests and linting:
   ```bash
   just test
   just fmt
   just lint
   ```
5. Commit your changes (`git commit -am 'Add new feature'`)
6. Push to your fork (`git push origin feature/my-feature`)
7. Open a Pull Request

## Code Style

- Use `cargo fmt` to format code
- Run `cargo clippy` to check for issues
- Follow existing code conventions

## Testing

Run tests with:
```bash
just test
```

## Build

Build the project with:
```bash
just build
```