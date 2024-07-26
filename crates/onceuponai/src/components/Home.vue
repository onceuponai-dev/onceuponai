<script setup lang="ts">
import { ref } from "vue";
// import { invoke } from "@tauri-apps/api/core";
// import { Command } from '@tauri-apps/plugin-shell';
// import { getCurrentWindow } from '@tauri-apps/api/window';
// import { exit } from '@tauri-apps/plugin-process';
import { fetch } from "../common";

// const appWindow = getCurrentWindow();
// appWindow.onCloseRequested(async (event) => {
//   event.preventDefault();
//   console.log('Cleaning up resources...');

//   // Example cleanup task
//   try {
//     bielik.value.kill();
//   } catch (e) {
//     console.warn('Failed to parse message', e);
//   }

//   // await someCleanupFunction();

//   await exit(0);
// });

// const greetMsg = ref("");

const hello_api = ref("");

// const bielik: any = ref(null);
// const sidecar_id: any = ref(null);

// async function greet() {
//   greetMsg.value = await invoke("config");
//   console.log(greetMsg.value)


//   const act: any = await invoke("spawn_actor");
//   console.log(act);
//   sidecar_id.value = act.sidecar_id;


//   // const command = Command.sidecar('binaries/sidecar/onceuponai-actors-candle', [
//   //   'spawn',
//   //   '-f',
//   //   "/home/jovyan/rust-src/onceuponai/examples/bielik.yaml"
//   // ]);
//   // bielik.value = await command.spawn();
//   // console.log(bielik.value.pid);
// }


// async function bielik_kill() {
//   const act = await invoke("kill_actor", {"sidecarId": sidecar_id.value});
//   console.log(act);


//   // bielik.value.kill();
// }

fetch(`/health`)
  .then(async (response: any) => {
    const t = await response.text();
    console.log(t);
    hello_api.value = t;
  })
  .catch(function (error: any) {
    console.log(error);
    hello_api.value = error;
  });


</script>

<template>


  <!-- <v-btn @click="greet">SPAWN</v-btn> -->
  <!-- <v-btn @click="bielik_kill">KILL</v-btn> -->
</template>
