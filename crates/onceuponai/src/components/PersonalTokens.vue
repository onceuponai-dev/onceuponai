<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { fetch } from "../common";

const dialog = ref(false);
const expirationDays = ref(1);
const patToken = ref("");
const patTokenDisplay = ref("");
const tooltipVisible = ref(false);

const openDialog = () => {
  dialog.value = true;
};

const closeDialog = () => {
  dialog.value = false;
};

onMounted(() => { });


const copyPat = () => {
  navigator.clipboard.writeText(patToken.value).then(() => {
    console.log('Text copied to clipboard');
    tooltipVisible.value = true;
    setTimeout(() => {
      tooltipVisible.value = false;
    }, 2000);
  }).catch((err) => {
    console.error('Failed to copy text: ', err);
  });
}

const generateToken = () => {
  fetch(`/api/user/personal-token`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        expiration_days: expirationDays.value,
      }),
    }
  )
    .then(async (response: any) => {
      const data = await response.json();
      console.log(data);
      patToken.value = data.personal_access_token;
      patTokenDisplay.value = `  ●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●`;

    })
    .catch(function (error) {
      console.log(error);
    });

};



</script>

<template>
  <v-container>
    <h1>Generate Personal Access Token</h1>
    <v-divider></v-divider>
    <br />
    <v-row>
      <v-col cols="2">
        <v-text-field v-model="expirationDays" type="number" label="Expiration days" />
      </v-col>
      <v-col cols="1">
        <v-btn size="large" @click="generateToken" icon="$keyPlus"></v-btn>
      </v-col>

      <v-col cols="5">
        <v-text-field v-model="patTokenDisplay" append-icon="$contentCopy" @click:append="copyPat"></v-text-field>
      </v-col>
      <v-col cols="1">
        <div v-if="tooltipVisible" class="tooltip">
          <span>copied</span>
        </div>
      </v-col>
    </v-row>
    <v-dialog v-model="dialog" max-width="500px">
      <v-card>
        <v-card-title>Model Details</v-card-title>
        <v-card-text>
          <!-- <p><strong>Host:</strong> {{ selectedModel?.host }}</p> -->
        </v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="primary" @click="closeDialog">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-container>


</template>

<style scoped>
.tooltip {
  margin-top: 15px;
}
</style>