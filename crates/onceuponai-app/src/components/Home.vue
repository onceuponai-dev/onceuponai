<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/tauri";
import axios from 'axios';
import axiosTauriApiAdapter from 'axios-tauri-api-adapter';
const client = axios.create({ adapter: axiosTauriApiAdapter });

const greetMsg = ref("");
const name = ref("");

const hello_api = ref("");

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  greetMsg.value = await invoke("greet", { name: name.value });
}

client.get(`http://localhost:8080/api/hello`)
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
    <button type="submit">HOME</button>
  </form>

  <p>{{ greetMsg }}</p>

  <p>API: {{ hello_api }}</p>
</template>
