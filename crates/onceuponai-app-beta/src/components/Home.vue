<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { fetch } from "../common";

const greetMsg = ref("");
const name = ref("");

const hello_api = ref("");

async function greet() {
  greetMsg.value = await invoke("config");
  console.log(greetMsg.value)
}

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


  <input id="greet-input" v-model="name" placeholder="Enter a name..." />
  <v-btn @click="greet">HOME</v-btn>

  <p>{{ greetMsg }}</p>

  <p>API: {{ hello_api }}</p>
</template>
