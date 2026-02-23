import "@/assets/scss/reset.scss"
import "@/assets/scss/base.scss"
import "./App.scss"

import { Messager, State } from "@denshya/reactive"
import { Proton } from "@denshya/proton"




interface User {
  status: string
  user_id: number
  username: string
  jwt: string
}
const user = new State<User | null>(null)



interface FSM {
  onEnter?(): void
  onExit?(): void
}

interface EffectCleanable {
  (): () => void
}

interface EffectSignal {
  (signal: AbortSignal): void
}

class Lifecycle {
  private readonly fsm: FSM = {}
  private abortController: AbortController | null = null

  constructor(fsm: FSM)
  constructor(effectCleanable: EffectCleanable)
  constructor(effectSignal: EffectSignal)
  constructor(arg: FSM | EffectCleanable | EffectSignal) {
    if (typeof arg === "function") {
      // EffectSignal
      if (arg.length === 1) {
        this.fsm.onEnter = () => {
          this.abortController?.abort()
          this.abortController = new AbortController

          arg(this.abortController.signal) as EffectSignal
        }
        this.fsm.onExit = () => {
          this.abortController?.abort()
          this.abortController = null
        }
        return
      }

      // EffectCleanable
      this.fsm.onEnter = () => {
        this.fsm.onExit = (arg as EffectCleanable)()
      }

      return
    }

    // FSM
    this.fsm = { ...arg }
  }

  enter() {
    this.fsm.onEnter?.()
  }
  exit() {
    this.fsm.onExit?.()
    this.fsm.onExit = undefined
  }
}
class LifecycleManager {
  private readonly items = new Set<Lifecycle>

  add(lifecycle: Lifecycle): void { this.items.add(lifecycle) }

  adopt(fsm: FSM): void
  adopt(effectCleanable: EffectCleanable): void
  adopt(effectSignal: EffectSignal): void
  adopt(arg: FSM | EffectCleanable | EffectSignal) { }
}



async function AppRoot(this: Proton.Component) {
  const uuid = new State<string | null>(null)
  const pending = new State(false)
  const status = new State<"" | null>(null)

  const socketSender = new Messager<{ type: string, payload: unknown }>()
  socketSender.subscribe(console.log)

  const mountLifecycle = new Lifecycle({
    onEnter: () => {
      const ws = new WebSocket(`wss://${window.location.host}/new`)
      ws.addEventListener("message", event => {
        const data = JSON.parse(event.data)

        if (data.status === "authenticated") {
          user.set(data)
          pending.set(false)
        }

        if (data.type === "ONE_TIME_AUTH") {
          uuid.set(data.token)
        }

        if (data.type === "AUTH_OK") {
          console.log(data)
          user.set({ username: "meow" })
          this.view.set(
            <div>{user.$.username}</div>
          )
        }
      })
      ws.addEventListener("close", () => pending.set(false))

      socketSender.subscribe(message => ws.send(JSON.stringify(message)))
    }
  })
  requestIdleCallback(() => mountLifecycle.enter())
  
  // () => pending.set(true)
  // () => pending.set(true)
  // pending.sink(true)
  return (
    <div>
      <div mounted={uuid}>
        {user.$.username}
        <a href={State.f`tg://resolve?domain=rukaku_bot&start=${uuid}`} on={{ click: () => pending.set(true) }}>
          <button>
            <span>Telegram</span>
          </button>
        </a>
        <form mounted={pending} on={{ submit: event => event.preventDefault() }}>
          <label>OTP:</label>
          <input name="otp" />
          <button on={{ click: event => {
            const button = event.currentTarget
            
            const otp = button.form.elements.otp.value.replace(" ", "")
            socketSender.dispatch({ type: "AUTH_OTP", payload: { otp: +otp } })
            button.textContent = "..."
          } }}>Submit</button>
        </form>
      </div>
    </div>
  )
}

export default AppRoot