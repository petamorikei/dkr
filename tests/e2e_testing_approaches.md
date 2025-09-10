# E2E Testing Approaches for TUI Applications (2024-2025)

## 1. **expectrl** - PTY-based Testing (Most Realistic)
```toml
[dev-dependencies]
expectrl = "0.7"
```

**Characteristics:**
- Controls interactive programs in a pseudo-terminal (PTY)
- Works like Don Libes' Expect
- Can spawn child applications and control them as if a human were typing
- Suitable for testing real terminal behavior

**Example:**
```rust
use expectrl::prelude::*;

#[test]
fn test_tui_navigation() -> Result<()> {
    let mut p = spawn("cargo run")?;
    
    // Wait for UI to load
    p.expect("Containers")?;
    
    // Test navigation
    p.send("2")?;  // Switch to Images tab
    p.expect("Images")?;
    
    p.send("j")?;  // Move down
    p.send("k")?;  // Move up
    
    p.send("q")?;  // Quit
    p.expect(Eof)?;
    
    Ok(())
}
```

## 2. **VHS** - Terminal Recording as Code
```yaml
# test.tape
Output demo.gif

Type "cargo run"
Sleep 2s
Type "2"  # Switch to Images
Sleep 1s
Type "j"  # Navigate down
Sleep 1s
Type "q"  # Quit
Sleep 1s
```

**Characteristics:**
- Write terminal interactions as code
- Generate GIFs/videos for documentation
- Can output ASCII/text for golden file testing
- Integrates with CI/CD pipelines
- Reproducible and version-controllable

**Usage:**
```bash
vhs test.tape
# Generates demo.gif and can output .txt for comparison
```

## 3. **Integration Testing Pattern** (Mock Terminal)
Based on 2024 community practices:

```rust
// Define Terminal trait
pub trait Terminal {
    fn poll_event(&self, duration: Duration) -> Result<bool>;
    fn read_event(&self) -> Result<Event>;
}

// Mock implementation for testing
struct MockTerminal {
    events: Receiver<KeyCode>,
}

impl Terminal for MockTerminal {
    fn read_event(&self) -> Result<Event> {
        let key = self.events.recv()?;
        Ok(Event::Key(KeyEvent::new(key, KeyModifiers::empty())))
    }
}

// Test example
#[tokio::test]
async fn test_container_operations() {
    let (tx, rx) = mpsc::channel();
    let terminal = MockTerminal { events: rx };
    
    // Send test events
    tx.send(KeyCode::Char('j')).unwrap();  // Navigate
    tx.send(KeyCode::Char('s')).unwrap();  // Stop container
    tx.send(KeyCode::Char('q')).unwrap();  // Quit
    
    // Run app with mock terminal
    // Assert expected behavior
}
```

## 4. **Ratatui TestBackend + Snapshot Testing**
```toml
[dev-dependencies]
insta = "1.34"
```

```rust
use ratatui::backend::TestBackend;
use insta::assert_snapshot;

#[test]
fn test_ui_rendering() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Render UI
    terminal.draw(|f| {
        render(f, &app, &containers).unwrap();
    }).unwrap();
    
    // Convert buffer to string
    let output = buffer_to_string(terminal.backend().buffer());
    
    // Snapshot test
    assert_snapshot!(output);
}
```

## 5. **Hybrid Approach: Script-driven Testing**
Combine shell scripts with test harness:

```bash
#!/bin/bash
# e2e_test.sh

# Start application in background
cargo run &
APP_PID=$!

# Use expect or similar tool
expect <<EOF
spawn nc localhost 8080  # If app has debug port
expect "Ready"
send "j\r"
expect "Selected: 1"
send "q\r"
expect eof
EOF

kill $APP_PID
```

## 6. **Container-based Testing (Maelstrom)**
Mentioned in 2024 as a fast test runner that runs every test in its own container:
- Isolates test environment
- Parallel test execution
- Reproducible results

## Recommended Strategy for dkr

### For CI/CD:
1. **VHS** for visual regression testing and documentation
2. **expectrl** for behavioral E2E tests

### For Development:
1. **TestBackend** with snapshot testing for UI components
2. **Mock Terminal** pattern for event handling

### Example Test Suite Structure:
```
tests/
├── e2e/
│   ├── navigation.rs      # expectrl tests
│   ├── operations.rs      # expectrl tests
│   └── tapes/            # VHS tape files
├── integration/
│   ├── mock_terminal.rs  # Mock-based tests
│   └── docker_mock.rs    # Docker API mocks
└── snapshots/            # insta snapshots
```

## Key Insights from 2024-2025

1. **expectrl** remains the most realistic E2E testing tool for TUI apps
2. **VHS** emerged as a popular tool for both testing and documentation
3. Community is moving towards trait-based abstractions for testability
4. Snapshot testing with **insta** is becoming standard for UI regression
5. Container-based test runners are gaining traction for isolation

## Comparison Table

| Tool | Real Terminal | CI/CD Ready | Speed | Setup Complexity |
|------|--------------|-------------|-------|-----------------|
| expectrl | ✅ | ✅ | Medium | Medium |
| VHS | ✅ | ✅ | Slow | Low |
| Mock Terminal | ❌ | ✅ | Fast | High |
| TestBackend | ❌ | ✅ | Fast | Low |
| Shell/Expect | ✅ | ⚠️ | Medium | Medium |

## Future Trends (2025)

- More sophisticated PTY emulation libraries
- Better integration between Ratatui and testing frameworks
- AI-assisted test generation from terminal recordings
- Standardized testing patterns in the Ratatui ecosystem