<script setup lang="ts">
import { ref, onMounted } from "vue";
import { fetch } from "../common";
import { useRouter } from 'vue-router'
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';

interface Actor {
  uuid: string;
  kind: string;
  metadata: ActorMetadata;
}

interface ActorMetadata {
  name: string;
  actor_id: string;
  actor_host: string;
  actor_seed: string;
  sidecar_id: string;
  features: string[];
}

const dialog: any = ref(false);
const selectedModel = ref<Actor | null>(null);
const actors = ref<Actor[]>([]);

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
}

async function kill(actor: Actor) {
  if (actor?.metadata?.sidecar_id != null) {
    const act = await invoke("kill_actor", { "sidecarId": actor.metadata?.sidecar_id });
    console.log(act);
  }
}

const headers = [
  { title: 'Kind', value: 'kind' },
  { title: 'Name', value: 'metadata.name' },
  { title: 'Host', value: 'metadata.actor_host' },
  { text: 'Actions', value: 'actions', sortable: false },
];


const openDialog = (model: Actor) => {
  selectedModel.value = model;
  dialog.value = true;
};

const closeDialog = () => {
  dialog.value = false;
};


onMounted(() => {
  refresh();
});

listen('message', async (event) => {
  const ev: any = event.payload;
  const payload: any = JSON.parse(ev);
  snackbarText.value = payload.message;
  console.log(payload);
  console.log(snackbarText.value)
  switch (payload.level) {
    case 'Success':
      snackbarColor.value = "success";
      break;
    case 'Info':
      snackbarColor.value = "white";
      break;
    case 'Error':
      snackbarColor.value = "red";
      break;
    default:
      snackbarColor.value = "white";
  }

  console.log(snackbarColor.value)
  snackbar.value = true;
  await new Promise(r => setTimeout(r, 2000));
  await refresh();
});

const router = useRouter();
// router.push("/")
const navigate = (route: string) => {
  router.push(route);
};
//unlisten()

const snackbar: any = ref(null);
const snackbarText: any = ref(null);
const snackbarColor: any = ref(null);

</script>

<template>
  <v-container>
    <h1>Active Actors</h1>
    <v-btn @click="refresh">REFRESH</v-btn>
    <v-btn @click="spawn">SPAWN</v-btn>

    <v-divider></v-divider>


    <v-container fluid>
      <v-row>
        <v-col v-for="item in actors" :key="item.kind" cols="12" sm="6" md="4">
          <v-card>
            <!-- <v-card-title>
              {{ item.metadata.name }}
            </v-card-title>
            <v-card-subtitle>
              {{ item.kind }}
            </v-card-subtitle> -->
            <v-card-text class="text-center">

              <div>{{ item.kind }}</div>

              <p class="text-h4 font-weight-black">{{ item.metadata.name }}</p>
              <v-divider></v-divider>
            </v-card-text>
            <v-card-actions>
              <v-btn @click="openDialog(item)" variant="tonal" color="blue-darken-3" block><b>Details</b></v-btn>
            </v-card-actions>
            <v-card-actions v-if="item.metadata.features.includes('chat')">
              <v-btn @click="navigate('/chat')" color="green-darken-3" variant="tonal" block><b>Chat</b></v-btn>
            </v-card-actions>
            <v-card-actions v-if="item.metadata.sidecar_id">
              <v-btn @click="kill(item)" variant="tonal" color="red-darken-3" block><b>Kill</b></v-btn>
            </v-card-actions>

          </v-card>
        </v-col>
      </v-row>
    </v-container>




    <v-dialog v-model="dialog" max-width="500px">
      <v-card>
        <v-card-title>Actor Details</v-card-title>
        <v-card-text>
          <v-divider></v-divider>
          <br />
          <p><strong>Kind:</strong> {{ selectedModel?.kind }}</p>
          <p><strong>Name:</strong> {{ selectedModel?.metadata.name }}</p>
          <p><strong>ActorId:</strong> {{ selectedModel?.metadata.actor_id }}</p>
          <p><strong>ActorHost:</strong> {{ selectedModel?.metadata.actor_host }}</p>
          <p><strong>ActorSeed:</strong> {{ selectedModel?.metadata.actor_seed }}</p>
          <p><strong>SidecarId:</strong> {{ selectedModel?.metadata.sidecar_id }}</p>
          <p><strong>Features:</strong> {{ selectedModel?.metadata.features }}</p>
        </v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="primary" @click="closeDialog">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
    <v-snackbar v-model="snackbar" :timeout="3000" :color="snackbarColor" bottom>
      {{ snackbarText }}
      <!-- <v-btn color="white" @click="snackbar = false">
          Close
        </v-btn> -->
    </v-snackbar>

  </v-container>
</template>
