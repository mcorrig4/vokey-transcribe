Authenticate Codex by forwarding localhost 1455 for OpenAI callback.

gh codespace ssh -c literate-invention-wrj5r57g4x29gwq -- -L 1455:localhost:1455


# Setup
- install rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
- set default nvm node to 22 (for convex support)
    ???
- install claude code cli
    curl -fsSL https://claude.ai/install.sh | bash