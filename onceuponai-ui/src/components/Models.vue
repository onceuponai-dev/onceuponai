<script lang="ts">
import { defineComponent, ref, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import axios from 'axios';

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

export default defineComponent({
  name: 'Stories',
  components: {
  },
  setup() {
    const router = useRouter();

    const dialog = ref(false);
    const selectedModel: any = ref<Actor | null>(null);
    const actors: any = ref<Actor[]>([]);

    axios.get(`/api/actors`)
      .then(function (response) {
        var values = Object.keys(response.data).map(function (key) {
          return response.data[key];
        });

        console.log(values);
        actors.value = values;
      })
      .catch(function (error) {
        console.log(error);
      });


    const headers = [
      { title: 'UUID', value: 'uuid' },
      { title: 'Kind', value: 'kind' },
      { title: 'Name', value: 'metadata.name' },
      { title: 'Host', value: 'metadata.actor_host' },
      { title: 'Seed', value: 'metadata.actor_seed' },
      { title: 'Features', value: 'metadata.features' },
      { text: 'Actions', value: 'actions', sortable: false },
    ];


    const openDialog = (model: Actor) => {
      selectedModel.value = model;
      dialog.value = true;
    };

    const closeDialog = () => {
      dialog.value = false;
    };

    onMounted(() => { });


    return {
      router,
      headers,
      actors,
      openDialog,
      closeDialog,
      selectedModel,
      dialog
    };

  }
});
</script>

<template>
  <v-container>
    <h1>Active Actors</h1>
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
          <v-btn color="primary" text @click="closeDialog">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-container>


</template>

<style scoped></style>