# rukaku-auth

## Environment Variables
You need a bot token from @BotFather.

## Running
1.  Run the Rust app: `cargo run`
2.  Open `index.html` in your browser.
3.  **Crucial**: Edit `index.html` line 38 and replace `YOUR_ACTUAL_BOT_USERNAME` with your bot's username (without the `@`).
4.  Click the button in the browser. It will open Telegram.
5.  Click "Start" in Telegram.
6.  Watch the HTML page update instantly without refreshing.

### Key Implementation Details

1.  **DashMap**: This is the "industry standard" for concurrent hashmaps in Rust. We use it to store `mpsc::Sender`. This allows the Bot thread (which is unrelated to the Web thread) to "inject" a message into the WebSocket stream of a specific user.
2.  **Actor-like Pattern**: In `ws_handler`, we spawn a `send_task` and a `recv_task`. The `send_task` listens to an *internal* Tokio channel. The Bot writes to this internal channel. This isolates the WebSocket logic from the Bot logic.
3.  **Deep Linking**: Telegram strictly uses `?start=<payload>` for deep linking. The bot receives this as `/start <payload>`. The code parses this payload to find the matching WebSocket session.