<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { fetch } from "../common";
import { useRouter } from 'vue-router'
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { load, dump } from 'js-yaml';

// interfaces
interface Actor {
  uuid: string;
  kind: string;
  metadata: ActorMetadata;
}

interface ActorSpecItem {
  key: string;
  value: any;
  type: string;
}

interface ActorMetadata {
  name: string;
  actor_id: string;
  actor_host: string;
  actor_seed: string;
  sidecar_id: string;
  features: string[];
}

interface Template {
  id: string;
  sidecar: string;
  kind: string;
  device: string;
  metadata: {
    name: string;
    description: string;
    url: string;
  };
  spec: ActorSpecItem[];
}

interface GalleryItem {
  id: string;
  template: string | null;
  device?: string;
  metadata: {
    name: string;
    description?: string;
    url?: string;
  };
  spec: ActorSpecItem[];
}

interface ModelsYaml {
  templates: Template[];
  galery: GalleryItem[];
}

const router = useRouter();

// refs

const webUI = ref("");
const dialog: any = ref(false);
const selectedModel = ref<Actor | null>(null);
const actors = ref<Actor[]>([]);
const snackbar: any = ref(null);
const snackbarText: any = ref(null);
const snackbarColor: any = ref(null);


const actorsGallery: any = ref(null);

const remoteSpawnConfig: any = ref(null);

const initCommand: any = ref(null);
const remoteSpawnCommand: any = ref(null);
const remoteSpawnDialog: any = ref(null);
const spawnDialog: any = ref(null);
const spawnActorName: any = ref("");
const spawnActorSidecar: any = ref("");
const spawnActorKind: any = ref("");
const spawnActorSpec: any = ref([]);
const spawnActorDevice: any = ref("cpu");
const spawnActorDevices = ['cpu', 'cuda'];
const spawnActorsTypes = ['string', 'number', 'bool', 'secret'];
const spawnActorNewPairType = ref("string");

const spawnSearchResults: any = ref([]);
const spawnSelectedSearch = ref('');
const spawnInProgress = ref(false);
const initInProgress = ref(false);


// functions
const mergeSpecs = (templateSpecs: ActorSpecItem[], gallerySpecs: ActorSpecItem[]): ActorSpecItem[] => {
  const mergedSpecs = [...templateSpecs];

  gallerySpecs.forEach(gallerySpec => {
    const index = mergedSpecs.findIndex(spec => spec.key === gallerySpec.key);
    if (index !== -1) {
      mergedSpecs[index] = gallerySpec;
    } else {
      mergedSpecs.push(gallerySpec);
    }
  });

  return mergedSpecs;
}

const createModelsList = (yamlString: string): any[] => {
  const modelsYaml = load(yamlString) as ModelsYaml;

  const models = modelsYaml.galery.map(galleryItem => {
    if (galleryItem.template === null) {
      // If template is null, return the gallery item as is
      return {
        id: galleryItem.id,
        device: galleryItem.device || 'cpu',
        metadata: {
          name: galleryItem.metadata.name,
          description: galleryItem.metadata.description || '',
          url: galleryItem.metadata.url || '',
        },
        spec: galleryItem.spec,
      };
    }

    const template = modelsYaml.templates.find(t => t.id === galleryItem.template);

    if (!template) {
      throw new Error(`Template with id ${galleryItem.template} not found`);
    }

    const mergedSpecs = mergeSpecs(template.spec, galleryItem.spec);

    return {
      ...template,
      id: galleryItem.id,
      device: galleryItem.device || template.device, // Use device from gallery item or fallback to template
      metadata: {
        ...template.metadata,
        name: galleryItem.metadata.name,
        description: galleryItem.metadata.description ?? template.metadata.description, // Use description from gallery item or fallback to template
        url: galleryItem.metadata.url ?? template.metadata.url, // Use URL from gallery item or fallback to template
      },
      spec: mergedSpecs,
    };
  });

  return models;
}

const refresh = async () => {
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

};

const buildActorJsonConfig = () => {
  const spec: any = {};
  spawnActorSpec.value.forEach((pair: ActorSpecItem) => {
    console.log(pair.key, pair.type, pair.value);
    const value = parseSpecItem(pair);
    spec[pair.key] = value;
  });

  spec["device"] = spawnActorDevice.value;

  const actorConfig = {
    "kind": spawnActorKind.value,
    "metadata": {
      "name": spawnActorName.value,
      "actor_host": ""
    },
    "spec": spec
  };

  console.log('Configuration data:', actorConfig);
  const jsonString = JSON.stringify(actorConfig);
  const specJsonBase64 = btoa(jsonString);
  return specJsonBase64;
}

const init = async () => {
  const specJsonBase64 = buildActorJsonConfig();
  initInProgress.value = true;
  const act: any = await invoke("init_actor", { "device": spawnActorDevice.value, "specJsonBase64": specJsonBase64 });
  console.log(act);
  initInProgress.value = false;
};


const spawn = async () => {
  const specJsonBase64 = buildActorJsonConfig();
  const act: any = await invoke("spawn_actor", { "name": spawnActorName.value, "device": spawnActorDevice.value, "specJsonBase64": specJsonBase64 });
  console.log(act);

  spawnDialog.value = false;
  spawnInProgress.value = true;
};

const kill = async (actor: Actor) => {
  if (actor?.metadata?.sidecar_id != null) {
    const act = await invoke("kill_actor", { "sidecarId": actor.metadata?.sidecar_id });
    console.log(act);
  }
};

const openDialog = (model: Actor) => {
  selectedModel.value = model;
  dialog.value = true;
};

const openRemoteSpawnDialog = () => {
  remoteSpawnDialog.value = true;
  const spec: any = {};
  spawnActorSpec.value.forEach((pair: ActorSpecItem) => {
    const value = parseSpecItem(pair);
    spec[pair.key] = value;
  });

  spec["device"] = spawnActorDevice.value;

  const config = {
    "kind": spawnActorKind.value,
    "metadata": {
      "name": spawnActorName.value,
      "actor_host": "127.0.0.1:1993",
      "actor_seed": "127.0.0.1:1992"
    },
    "spec": spec
  };

  remoteSpawnConfig.value = dump(config);
  remoteSpawnCommand.value = `${spawnActorSidecar.value}-${spec["device"]} spawn -f config.yaml`

  const initConfig = buildActorJsonConfig();
  initCommand.value = `${spawnActorSidecar.value}-${spawnActorDevice.value} init -j ${initConfig}`


};


const closeDialog = () => {
  dialog.value = false;
};

onMounted(async () => {
  refresh();
  const ag = await fetch(`/api/actors/gallery`);
  const gallery = await ag.text();
  actorsGallery.value = createModelsList(gallery);
  console.log(actorsGallery.value)
  spawnSearchResults.value = actorsGallery.value.map((a: any) => a.id);
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
  spawnInProgress.value = false;
  await refresh();
});

const navigate = (route: string) => {
  router.push(route);
};


const addPair = () => {
  spawnActorSpec.value.push({ key: '', value: '' });
};

const removePair = (index: any) => {
  spawnActorSpec.value.splice(index, 1);
};

const getInputComponent = (type: any) => {
  switch (type) {
    case 'number':
      return 'v-text-field';
    case 'secret':
      return 'v-text-field';
    case 'bool':
      return 'v-checkbox';
    default:
      return 'v-text-field';
  }
};

const getInputLabel = (type: any) => {
  switch (type) {
    case 'number':
      return 'Number';
    case 'bool':
      return 'Boolean';
    case 'secret':
      return 'Secret';
    default:
      return 'Value';
  }
};

const parseSpecItem = (item: ActorSpecItem) => {

  if(item.value == null) {
    return item.value;
  }

  switch (item.type) {
    case 'number':
      return Number(String(item.value))
    case 'bool':
      return String(item.value).toLowerCase() == "true";
    default:
      return item.value;
  }
};


const getInputType = (type: any) => {
  switch (type) {
    case 'number':
      return 'number';
    case 'secret':
      return 'password';
    default:
      return 'text';
  }
};

const onSearch = () => {
};

const config: any = await invoke("config");
const copyUrl = () => {
  webUI.value = `${config.base_url}/login?token=${config.auth_token}`;
  navigator.clipboard.writeText(webUI.value).then(() => {
    console.log('Text copied to clipboard');
  }).catch((err) => {
    console.error('Failed to copy text: ', err);
  });
}


watch(spawnSelectedSearch, (newValue) => {
  console.log("NEW ITEM" + newValue);
  const selectedItem = actorsGallery.value.find((item: any) => item.id === newValue);
  if (selectedItem) {
    spawnActorKind.value = selectedItem.kind;
    spawnActorName.value = selectedItem.metadata.name;
    spawnActorSidecar.value = selectedItem.sidecar;
    spawnActorSpec.value = selectedItem.spec;
    spawnActorDevice.value = selectedItem.device;
    console.log(selectedItem)
  }
});

</script>

<template>
  <v-container>
    <!-- <h1>Active Actors</h1> -->
    <v-btn @click="refresh" prepend-icon="$refresh" variant="text">REFRESH</v-btn>
    &nbsp;
    <v-btn @click="spawnDialog = true" prepend-icon="$power" variant="text">SPAWN</v-btn>
    &nbsp;
    <v-btn @click="copyUrl" prepend-icon="$web" variant="text">COPY WEB URL</v-btn>


    <v-divider></v-divider>


    <v-container fluid>
      <v-row>
        <v-col cols="12" sm="6" md="4" v-if="spawnInProgress">
          <v-card>
            <v-card-text class="text-center">
              <div>{{ spawnActorKind.value }}</div>

              <p class="text-h4 font-weight-black">{{ spawnActorName.value }}</p>
              <br />
              <br />
              <br />
              <br />
              ACTOR LOADING ...
              <br />
              <br />
              <v-icon icon="$brain" size="x-large" class="rotating"></v-icon>
              <!-- <v-img src="/images/brain.gif" width="100" class="centered-image"></v-img> -->
              <br />
              <br />
              <br />
              <br />
              <br />

            </v-card-text>


          </v-card>
        </v-col>

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

              <p class="text-h6 font-weight-black">{{ item.metadata.name }}</p>
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

    <v-dialog v-model="remoteSpawnDialog" max-width="800px">
      <v-card>
        <v-card-title>Actor Config</v-card-title>
        <v-card-text>
          <v-divider></v-divider>
          <v-stepper :items="['Initialize', 'Spawn']">
            <template v-slot:item.1>
              <v-card title="Initialize actor" flat>
                <v-card-text>
                  <span><i>{{ initCommand }}</i></span>
                </v-card-text>
              </v-card>
            </template>
            <template v-slot:item.2>
              <v-card title="Spawn actor" flat>
                <v-card-text>
                  <v-textarea label="config.yaml" rows="12" v-model="remoteSpawnConfig"></v-textarea>
                  <span><i>{{ remoteSpawnCommand }}</i></span>
                </v-card-text>
              </v-card>
            </template>
          </v-stepper>
        </v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="primary" @click="remoteSpawnDialog = false">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>


    <v-dialog v-model="spawnDialog" width="90%">
      <v-card>
        <v-card-title>
          <span class="headline">Spawn Actor</span>
        </v-card-title>
        <v-card-text>
          <v-form>
            <v-autocomplete v-model="spawnSelectedSearch" :items="spawnSearchResults" label="Search"
              placeholder="Type to search..." @input="onSearch" item-text="name" item-value="name"
              class="mb-4"></v-autocomplete>
            <v-divider></v-divider>
            <br />
            <v-text-field v-model="spawnActorKind" label="Kind" required></v-text-field>
            <v-text-field v-model="spawnActorName" label="Name" required></v-text-field>

            <v-divider></v-divider>
            <br />
            <div v-for="(pair, index) in spawnActorSpec" :key="index" class="d-flex align-center mb-2">
              <v-text-field v-model="pair.key" label="Key" class="mr-2 key-field" required></v-text-field>
              <!-- <v-text-field v-model="pair.value" label="Value" required></v-text-field> -->
              <component :is="getInputComponent(pair.type)" v-model="pair.value" :label="getInputLabel(pair.type)"
                :type="getInputType(pair.type)" required class="flex-grow-1 mr-2"></component>
              <v-btn icon @click="removePair(index)" variant="text">
                <v-icon color="red">$delete</v-icon>
              </v-btn>
            </div>
            <div class="d-flex align-center mb-2">
              <v-select v-model="spawnActorNewPairType" :items="spawnActorsTypes" max-width="300px" label="Select Type"
                class="ml-2"></v-select>
              <v-btn color="primary" @click="addPair" variant="text" size="large" style="margin-top: -15px;">Add Spec
                Item</v-btn>
            </div>
            <br /><br />
            <v-divider></v-divider>
            <br />

            <v-select v-model="spawnActorDevice" :items="spawnActorDevices" label="Device" required></v-select>
          </v-form>
          <br />
        </v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-progress-circular color="orange" v-if="initInProgress" indeterminate></v-progress-circular>
          <v-btn color="orange darken-1" @click="init"><b>Download</b></v-btn>
          <v-btn color="green darken-1" @click="spawn"><b>Spawn</b></v-btn>
          <v-btn color="blue darken-1" @click="openRemoteSpawnDialog"><b>Config</b></v-btn>
          <v-btn color="grey darken-1" @click="spawnDialog = false"><b>Cancel</b></v-btn>
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


<style scoped>
.key-field {
  max-width: 200px !important;
}

@keyframes blink {
  0% {
    opacity: 1;
  }

  50% {
    opacity: 0.5;
  }

  100% {
    opacity: 1;
  }
}

.blinking {
  animation: blink 2s infinite;
}

@keyframes rotate {
  0% {
    transform: rotate(0deg);
  }

  100% {
    transform: rotate(360deg);
  }
}

.rotating {
  animation: rotate 2s linear infinite;
}

.centered-image {
  display: block;
  margin: 0 auto;
}
</style>
