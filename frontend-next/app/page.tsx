"use client";

import { useState } from "react";

// The Nacharium node API base. In prod set NEXT_PUBLIC_NACHA_API.
const API =
  process.env.NEXT_PUBLIC_NACHA_API || "http://100.65.135.62:8787/api/v1";

type Me = { veil_did: string; state?: string; tokens?: number };

export default function Home() {
  const [me, setMe] = useState<Me | null>(null);
  const [nick, setNick] = useState("");
  const [wallet, setWallet] = useState<number | null>(null);
  const [log, setLog] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);

  const say = (m: string) => setLog((l) => [m, ...l].slice(0, 8));

  async function enterVeil() {
    setBusy(true);
    try {
      const r = await fetch(`${API}/veil`, { method: "POST" });
      const d = await r.json();
      setMe({ veil_did: d.veil_did, state: "seven" });
      setWallet(null);
      say(`Entered the Veil → ${d.veil_did.slice(0, 22)}… (Genesis Seven)`);
    } catch {
      say("⚠ cannot reach node — check Tailscale / NEXT_PUBLIC_NACHA_API");
    } finally {
      setBusy(false);
    }
  }

  async function register() {
    if (!me || !nick) return;
    setBusy(true);
    try {
      const r = await fetch(`${API}/veil/register`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ veil_did: me.veil_did, nick }),
      });
      const d = await r.json();
      say(`Registered "${nick}" → granted ${d.tokens_granted} NARI`);
      const w = await fetch(`${API}/wallet/${me.veil_did}`).then((x) => x.json());
      setWallet(w.balance_nari);
      setMe({ ...me, state: "registered", tokens: d.tokens_granted });
    } catch {
      say("⚠ register failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="min-h-screen bg-[#0A0A0F] text-[#E8E6F0] flex flex-col items-center px-5 py-14">
      <div className="text-6xl font-bold bg-gradient-to-r from-[#7C6AFF] to-[#4FD1E0] bg-clip-text text-transparent">
        ∞
      </div>
      <h1 className="tracking-[4px] text-3xl mt-2">NACHARIUM</h1>
      <p className="text-[#9a96b0] italic mt-1">The world is flat. Now build it.</p>
      <span className="mt-3 text-xs text-[#5EE6A8] bg-[#5ee6a826] px-3 py-1 rounded-full">
        ● Epoch 1 · live client
      </span>

      <div className="mt-8 w-full max-w-md bg-[#141420] border border-white/10 rounded-2xl p-6">
        <h2 className="text-[#4FD1E0] text-sm mb-3">Enter Nacharium</h2>
        <button
          onClick={enterVeil}
          disabled={busy}
          className="w-full py-3 rounded-lg font-medium text-[#0A0A0F] bg-gradient-to-r from-[#7C6AFF] to-[#4FD1E0] disabled:opacity-50"
        >
          {me ? "Re-enter the Veil" : "Enter the Veil (Genesis Seven)"}
        </button>

        {me && (
          <div className="mt-4 text-sm">
            <div className="text-[#9a96b0] break-all">
              DID: <span className="text-[#E8E6F0]">{me.veil_did}</span>
            </div>
            <div className="text-[#9a96b0] mt-1">
              State: <span className="text-[#4FD1E0]">{me.state}</span>
              {wallet !== null && (
                <> · Wallet: <span className="text-[#5EE6A8]">{wallet} NARI</span></>
              )}
            </div>
            {me.state === "seven" && (
              <div className="mt-4 flex gap-2">
                <input
                  value={nick}
                  onChange={(e) => setNick(e.target.value)}
                  placeholder="choose a nick"
                  className="flex-1 bg-[#0A0A0F] border border-white/10 rounded-lg px-3 py-2 text-sm outline-none focus:border-[#7C6AFF]"
                />
                <button
                  onClick={register}
                  disabled={busy || !nick}
                  className="px-4 rounded-lg bg-white/10 text-sm disabled:opacity-40"
                >
                  Register → 13 NARI
                </button>
              </div>
            )}
          </div>
        )}
      </div>

      {log.length > 0 && (
        <div className="mt-6 w-full max-w-md bg-[#141420] border border-white/10 rounded-2xl p-4 text-xs font-mono text-[#9a96b0]">
          {log.map((l, i) => (
            <div key={i} className="py-1 border-b border-white/5 last:border-0">
              {l}
            </div>
          ))}
        </div>
      )}

      <p className="mt-10 text-[#9a96b0] text-xs italic">
        ∞ Sáng thế bởi Mula · node: {API}
      </p>
    </main>
  );
}
