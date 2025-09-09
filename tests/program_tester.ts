import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ProgramTester } from "../target/types/program_tester";

describe("program_tester", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.programTester as Program<ProgramTester>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });

  it("Emits Received event", async () => {
    const value = new anchor.BN(42);

    const listener = program.addEventListener("received", (event, slot) => {
      console.log("Received event at slot", slot, event);
    });

    const sig = await program.methods.emitReceived(value).rpc();
    console.log("emit_received tx", sig);

    await program.removeEventListener(listener);

    
  });
});
