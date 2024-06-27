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
    const patToCopy = ref(null);

    const openDialog = () => {
      dialog.value = true;
    };

    const closeDialog = () => {
      dialog.value = false;
    };

    onMounted(() => { });


    const copyPat = () => {
      console.log(patToken.value);
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
      copyPat,
      patToCopy
    };

  }
});
</script>

<template>
  <v-container>
    <h1>Generate Personal Access Token</h1>
    <v-divider></v-divider>
    <v-btn @click="generateToken">Generate</v-btn>
    <v-text-field v-model="expirationDays" type="number" label="Expiration days" />
    <v-text-field v-model="patToken" append-icon="mdi-content-copy" @click:append="copyPat" ></v-text-field>
    <v-dialog v-model="dialog" max-width="500px">
      <v-card>
        <v-card-title>Model Details</v-card-title>
        <v-card-text>
          <!-- <p><strong>Host:</strong> {{ selectedModel?.host }}</p> -->
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