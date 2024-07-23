<script setup lang="ts">
import { ref, onMounted } from "vue";
import { fetch } from "../common";
import { invoke } from "@tauri-apps/api/core";

interface Actor {
  uuid: string;
  kind: string;
  metadata: ActorMetadata;
}

interface ActorMetadata {
  name: string;
  actor_host: string;
  actor_seed: string;
}

const dialog: any = ref(false);
const selectedModel: any = ref<Actor | null>(null);
const actors: any = ref<Actor[]>([]);

async function refresh() {
  fetch(`/api/actors`)
    .then(async (response: any) => {
      const data = await response.json();
      var values = Object.keys(data).map(function (key) {
        return data[key];
      });

      console.log(values);
      actors.value = values;
    })
    .catch(function (error: any) {
      console.log(error);
    });

}

const sidecar_id: any = ref(null);

async function spawn() {

  const bielik = {
    "kind": "quantized",
    "metadata": {
      "name": "bielik",
      "actor_host": ""
    },
    "spec": {
      "model_repo": "speakleash/Bielik-7B-Instruct-v0.1-GGUF",
      "model_file": "bielik-7b-instruct-v0.1.Q4_K_S.gguf",
      "tokenizer_repo": "speakleash/Bielik-7B-Instruct-v0.1",
      "device": "cuda"
    }
  };
  
  const jsonString = JSON.stringify(bielik);
  const specJsonBase64 = btoa(jsonString);
  const act: any = await invoke("spawn_actor", { "name": "bielik", "specJsonBase64": specJsonBase64 });
  console.log(act);
  sidecar_id.value = act.sidecar_id;
}

async function kill() {
  const act = await invoke("kill_actor", { "sidecarId": sidecar_id.value });
  console.log(act);
}




const headers = [
  { title: 'UUID', value: 'uuid' },
  { title: 'Kind', value: 'kind' },
  { title: 'Name', value: 'metadata.name' },
  { title: 'Host', value: 'metadata.actor_host' },
  { title: 'Seed', value: 'metadata.actor_seed' },
  { title: 'Features', value: 'metadata.features' },
  { text: 'Actions', value: 'actions', sortable: false },
];


const openDialog = (model: any) => {
  selectedModel.value = model;
  dialog.value = true;
};

const closeDialog = () => {
  dialog.value = false;
};


onMounted(() => {
  refresh();
});


</script>

<template>
  <v-container>
    <h1>Active Actors</h1>
    <v-btn @click="refresh">REFRESH</v-btn>
    <v-btn @click="spawn">SPAWN</v-btn>
    <v-btn @click="kill">KILL</v-btn>

    <v-divider></v-divider>
    <v-data-table :headers="headers" :items="actors" item-key="kind">
      <template v-slot:[`item.actions`]="{ item }">
        <v-btn @click="openDialog(item)" color="primary">Details</v-btn>
      </template>
    </v-data-table>

    <v-dialog v-model="dialog" max-width="500px">
      <v-card>
        <v-card-title>Model Details</v-card-title>
        <v-card-text>
          <p><strong>Name:</strong> {{ selectedModel?.name }}</p>
          <p><strong>State:</strong> {{ selectedModel?.state }}</p>
          <p><strong>Host:</strong> {{ selectedModel?.host }}</p>
        </v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="primary" @click="closeDialog">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-container>
</template>
