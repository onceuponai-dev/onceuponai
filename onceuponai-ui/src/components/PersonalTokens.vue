<script lang="ts">
import { defineComponent, ref, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import axios from 'axios';

export default defineComponent({
  name: 'PersonalTokens',
  components: {
  },
  setup() {
    const router = useRouter();

    const dialog = ref(false);
    const expirationDays = ref(1);
    const patToken = ref("");
    const patTokenDisplay = ref("");
    const patToCopy = ref(null);
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
      axios.post(`/api/user/personal-token`, {
        expiration_days: expirationDays.value,
      }, {
        headers: {
          'Content-Type': 'application/json'
        }
      })
        .then(function (response) {
          console.log(response);
          patToken.value = response.data.personal_access_token;
          patTokenDisplay.value = `  ●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●●`;

        })
        .catch(function (error) {
          console.log(error);
        });

    };



    return {
      router,
      openDialog,
      closeDialog,
      dialog,
      generateToken,
      expirationDays,
      patToken,
      patTokenDisplay,
      copyPat,
      patToCopy,
      tooltipVisible
    };

  }
});
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
        <v-btn size="large" @click="generateToken" icon="mdi-key-plus"></v-btn>
      </v-col>

      <v-col cols="5">
        <v-text-field v-model="patTokenDisplay" append-icon="mdi-content-copy" @click:append="copyPat"></v-text-field>
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