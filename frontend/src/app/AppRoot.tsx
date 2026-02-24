import "@/assets/scss/reset.scss"
import "@/assets/scss/base.scss"
import "./App.scss"

import { Messager, State, StateBoolean, StateLike } from "@denshya/reactive"
import { MountRoutine, Tama } from "@denshya/tama"




interface User {
  status: string
  user_id: number
  username: string
  jwt: string
}
const user = new State<User | null>(null)



class AuthSocket {
  readonly uuid = new State<string | null>(null)
  readonly pending = new State(false)

  readonly error = new State<Error | null>(null)
  readonly status = new State<"CONNECTING" | "OPEN" | "CLOSED" | null>(null)

  readonly sender = new Messager<{ type: string, payload: unknown }>()
  readonly receiver = new Messager<{ type: string, payload: unknown }>()

  readonly routine: MountRoutine

  constructor() {
    this.sender.subscribe(console.log)
    this.receiver.subscribe(console.log)

    this.routine = new MountRoutine(() => this.activate())
  }

  private activate() {
    this.status.set("CONNECTING")

    const ws = new WebSocket(`wss://auth.rukaku.com/new`)

    ws.addEventListener("message", event => this.onMessage(event))
    ws.addEventListener("open", () => this.status.set("OPEN"))
    ws.addEventListener("close", event => {
      if (!event.wasClean) {
        this.error.set(new Error("WebSocket Connection Closed: " + event.code))
      }

      this.status.set("CLOSED")
    })

    const socketSenderSub = this.sender.subscribe(message => ws.send(JSON.stringify(message)))

    return () => {
      ws.close()
      socketSenderSub.unsubscribe()
    }
  }

  private onMessage(event: MessageEvent) {
    console.log(event)
    const data = JSON.parse(event.data)

    if (data.status === "authenticated") {
      this.receiver.dispatch(data)
      this.pending.set(false)
    }

    if (data.type === "ONE_TIME_AUTH") {
      this.uuid.set(data.token)
    }

    if (data.type === "AUTH_OK") {
      this.receiver.dispatch(data)
    }
  }
}

async function AppRoot(this: Tama.Component) {
  const status = new State<"start" | null>(null)
  const authSocket = new AuthSocket


  return (
    <div>
      <pre>{authSocket.error}</pre>
      <div mounted={authSocket.uuid.is(null)}>
        <Button onClick={() => authSocket.routine.enter()} pending={authSocket.status.is("CONNECTING")}>Log in</Button>
      </div>
      <div mounted={authSocket.uuid}>
        {user.$.username}
        <a href={State.f`tg://resolve?domain=rukaku_bot&start=${authSocket.uuid}`} on={{ click: () => authSocket.pending.set(true) }}>
          <button>
            <span>Telegram</span>
          </button>
        </a>
        <form mounted={authSocket.pending} on={{ submit: event => event.preventDefault() }}>
          <label>OTP:</label>
          <input name="otp" />
          <button on={{ click: event => {
            const button = event.currentTarget
            const otp = button.form.elements.otp.value.replace(" ", "")

            authSocket.sender.dispatch({ type: "AUTH_OTP", payload: { otp: +otp } })
          } }}>Submit</button>
        </form>
      </div>

    </div>
  )
}


export default AppRoot

interface ButtonProps {
  onClick?(): void
  pending?: StateLike<boolean>
  children: unknown
}

function Button(props: ButtonProps) {
  const pending = new State(false)
  
  async function onClick() {
    try {
      pending.set(true)
      await props.onClick?.()
    } finally {
      pending.set(false)
    }
  }

  
  
  return (
    <button className="button" classMods={{ pending: StateBoolean.combine([pending, props.pending], (...bools) => bools.some(Boolean)) }} on={{ click: onClick }}>
      <span className="content">{props.children}</span>
      <span className="spinner" />
    </button>
  )
}