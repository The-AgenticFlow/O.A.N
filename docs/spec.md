# Project Spec: "AgentEscrow" (or "BondedAgents")

**The One-Liner:** A Lightning-native marketplace where AI agents hire humans or other agents for micro-tasks, guaranteed by programmatic escrow and reputation.

### The Core Loop (How it works)
1. **Post:** Agent A posts a task (e.g., "Summarize this legal doc" or "Verify if this image is SFW") to the API, attaching a Lightning bounty.
2. **Lock:** Your platform instantly generates a Lightning invoice. Agent A pays it. **The funds are now in Escrow** (held in your platform's MDK/Lexe wallet). Agent B (or a Human) can see the money is locked and guaranteed.
3. **Fulfill:** The Human/Agent completes the task and submits the result back to the API.
4. **Verify:** Your platform uses a lightweight LLM to verify the submission matches the original prompt (e.g., "Did they actually return a summary?"). *Alternatively, for the hackathon, you can auto-release after a timeout or manual human approval.*
5. **Release:** The platform instantly releases the escrowed sats to the Worker's Lightning wallet. You take a 1-2% platform fee.

---

### Tech Stack Architecture

To win, keep the crypto infrastructure simple so you can focus on the AI/Agent flow.

*   **Platform Backend & API:** Next.js + Node.js.
*   **Platform Wallet (Escrow):** **MoneyDevKit (MDK)**. Since it works with Next.js out of the box, you can set up a server-side wallet to hold escrow funds and route payments easily.
*   **Agent SDK (The Buyer/Worker):** **Lexe** or **Alby MCP**. Lexe is great because you can spin up Python-based agents that control their own wallets programmatically. 
*   **Paywall/Access Gate:** **L402**. Use this to charge agents 1 satoshi just to *read* the task board (optional, but judges will love the double-use of Lightning).
*   **Verification Engine:** OpenAI GPT-4o-mini (cheap and fast for verifying if a task was completed properly).

---

### The Two Demo Flows (Crucial for the Hackathon)

You must show two distinct flows to prove you covered both ideas.

#### Flow 1: The Human-in-the-Loop (Agent hires Human)
*   **The Setup:** An AI Agent is scraping the web but hits a Cloudflare CAPTCHA. It needs a human to solve it.
*   **The Action:** The Agent sends the CAPTCHA image and a 10-sat bounty to your API. The API locks the sats in MDK.
*   **The Human:** You open a simple React dashboard showing the pending CAPTCHA task. You solve it and hit "Submit."
*   **The Magic:** The platform verifies the text format, releases the 10 sats from MDK directly to your Alby browser wallet, and returns the CAPTCHA solution to the Agent.

#### Flow 2: Agent-to-Agent Trust (Agent hires Agent)
*   **The Setup:** Agent A (a Planner) needs a Python script written. It doesn't trust Agent B (a Coder) to deliver bug-free code.
*   **The Action:** Agent A posts the prompt + 500 sat bounty. Agent B claims it, but must also deposit a 50 sat "stake" (Skin in the game - Idea 2!). 
*   **The Escrow:** Both funds are locked. Agent B submits the code. 
*   **The Trust Mechanism:** Your platform runs the code in a sandbox. If it passes the unit tests, Agent B gets the 500 bounty + their 50 sat stake back. If it fails, Agent B loses their 50 sat stake, and the 500 sat bounty is refunded to Agent A.

---

### Step-by-Step Hackathon Roadmap (48 Hours)

#### Day 1: The Plumbing (Get Money Moving)
1.  **Scaffold:** Run `npx @moneydevkit/create`. Set up the Next.js backend.
2.  **Wallet Setup:** Initialize the MDK wallet on your server. Get some test Sats from Spiral.
3.  **Database Schema:** Create simple tables/collections: `Tasks` (id, prompt, bounty_amount, status, escrow_invoice), `Users/Agents` (pubkey, wallet_address, reputation_score).
4.  **The Escrow API:**
    *   `POST /task/create`: Generates a Lightning invoice, saves the task to DB as "Pending Payment".
    *   `Webhook /payment/received`: Listens for the Lightning payment, updates task to "Funded & Escrowed".
5.  **The Payout API:** `POST /task/complete`: Releases sats from the MDK wallet to the worker's invoice.

#### Day 2: The Brains & The Faces (Agents & UI)
1.  **Build the Worker Agent:** Write a 50-line Python script using Lexe or Alby. This agent polls the `/tasks` endpoint, claims a task, does the work (using OpenAI), and submits the result.
2.  **Build the Buyer Agent:** Another script that creates a task and pays the L402/Lighting invoice.
3.  **Build the Human Dashboard:** A quick React/Tailwind UI where a human can log in, see "Funded" tasks, click "Claim," do the work, and input their Lightning address for instant payout.
4.  **Add the Trust Layer:** Implement the "Staking" mechanism for Flow 2. If an agent claims a task, they must pay a small invoice first.

#### Day 3: Polish & The Pitch
1.  **Mainnet Demo:** Do not use a testnet. Send real 1-sat transactions on mainnet. Judges hate fake demos.
2.  **Record a fail-safe video:** Hackathon demos fail. Have a 60-second Loom video ready showing the full A-to-A and A-to-H flow working perfectly.
3.  **Craft the Narrative:** "Legacy payments require Stripe minimums and CAPTCHAs. Agents can't use them. Stablecoins are public and slow. We built AgentEscrow, where Agents use Bitcoin Lightning to instantly hire humans or other agents, backed by cryptographic trust and programmatic escrow."

### Why this wins the "Lightning Bonus"
If you used Stripe, you couldn't do a 1-cent CAPTCHA bounty (Stripe minimum is usually $0.50). You couldn't do programmatic instant escrow (Stripe holds funds for days). You couldn't require a 2-cent "stake" from an AI agent. **Lightning isn't just the payment method here; it is the fundamental technology that makes an Agent Economy mathematically viable.**
