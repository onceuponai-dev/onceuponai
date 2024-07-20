<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/tauri";
import {axios_client} from "../common";
const client = await axios_client();

const greetMsg = ref("");
const name = ref("");

const hello_api = ref("");

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  greetMsg.value = await invoke("config");
}

client.get(`/api/hello`)
  .then(function (response) {
    hello_api.value = response.data;
  })
  .catch(function (error) {

    hello_api.value = error;
  });


</script>

<template>


  <form class="row" @submit.prevent="greet">
    <input id="greet-input" v-model="name" placeholder="Enter a name..." />
    <v-btn type="submit">HOME</v-btn>
  </form>

  <p>{{ greetMsg }}</p>

  <p>API: {{ hello_api }}</p>
</template>
