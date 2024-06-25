<script lang="ts">
import { defineComponent, ref, onMounted } from 'vue';
import { useRouter } from 'vue-router';

interface Model {
  name: string;
  state: string;
  host: string;
}

export default defineComponent({
  name: 'Stories',
  components: {
  },
  setup() {
    const router = useRouter();

    const dialog = ref(false);
    const selectedModel: any = ref<Model | null>(null);

    const headers = [
      { text: 'Name', value: 'name' },
      { text: 'State', value: 'state' },
      { text: 'Host', value: 'host' },
      { text: 'Actions', value: 'actions', sortable: false },
    ];

    const models: any = ref<Model[]>([
      { name: 'Model 1', state: 'Active', host: 'Host 1' },
      { name: 'Model 2', state: 'Inactive', host: 'Host 2' },
      // Add more models as needed
    ]);

    const openDialog = (model: Model) => {
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
      models,
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
    <v-data-table :headers="headers" :items="models" item-key="name">
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

<style scoped>
.position-absolute {
  position: absolute;
  top: 40%;
  left: 50%;
  transform: translate(-50%, -50%);
}

.position-absolute-buttons {
  position: absolute;
  top: 70%;
  left: 50%;
  transform: translate(-50%, -50%);
}

.bangers-font {
  font-family: 'Bangers', cursive;
  font-size: 350%;
  padding: 7px;
  margin: 5px;
}

.video-container {
  position: relative;
  padding-bottom: 56.25%;
  /* 16:9 aspect ratio for widescreen videos */
  overflow: hidden;
}

.video-container iframe {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
}

.swanky-font {
  font-family: 'Fontdiner Swanky';
  font-size: 32;
}
</style>