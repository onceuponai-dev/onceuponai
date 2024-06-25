<script lang="ts">
import { defineComponent, ref, onMounted, nextTick } from 'vue';
import { useRouter } from 'vue-router';
import axios from 'axios';

interface Message {
  text: string;
  type: 'request' | 'response';
}

export default defineComponent({
  name: 'Chat',
  components: {
  },
  setup() {
    const inputMessage = ref('');
    const messages = ref<Message[]>([]);
    const chatArea = ref<HTMLElement | null>(null);
    const showProgress = ref(false);

    const sendMessage = () => {
      if (inputMessage.value.trim() === '') return;

      var text = inputMessage.value;
      messages.value.push({ text: text, type: 'request' });
      inputMessage.value = '';
      showProgress.value = true;
      axios.post(`/api/invoke/quantized/bielik`, {
        prompt: [text],
      }, {
        headers: {
          'Content-Type': 'application/json'
        }
      })
        .then(function (response) {

          showProgress.value = false;
          console.log(response);
          var result = response.data.results[0];
          messages.value.push({ text: result, type: 'response' });
          nextTick(() => {
            setTimeout(() => {
              var chatDiv = document.getElementsByClassName("chat-area")[0]
              chatDiv.scrollTop = chatDiv.scrollHeight;
            }, 100);
          });

        })
        .catch(function (error) {

          messages.value.push({ text: "😿 Error", type: 'response' });
          showProgress.value = false;
          console.log(error);
        });

    };

    onMounted(() => {
      if (chatArea.value) {
        chatArea.value.scrollTop = chatArea.value.scrollHeight;
      }
    });

    return {
      inputMessage,
      messages,
      sendMessage,
      chatArea,
      showProgress
    };


  }
});
</script>

<template>
  <v-container class="d-flex flex-column fill-height">
    <v-card class="flex-grow-1 mb-2 overflow-auto fill-width chat-area" ref="chatArea">
      <v-row v-for="(message, index) in messages" :key="index" class="mb-2">
        <v-col :cols="message.type === 'request' ? '8' : '12'" :offset="message.type === 'request' ? '0' : '1'">
          <v-card :class="message.type" class="pa-3 rounded-pill" width="90%">
            <v-card-text class="card-text">{{ message.text }}</v-card-text>
          </v-card>
        </v-col>
      </v-row>
      <v-row>
        <v-col cols="2" offset="6">
          <v-progress-circular v-if="showProgress" color="green" indeterminate></v-progress-circular>
        </v-col>
      </v-row>
    </v-card>
    <v-bottom-navigation color="primary" horizontal height="75">
      <v-row>
        <v-col cols="2" offset="1">
          <v-select label="Actor"  menu-icon="mdi-brain" bg-color="white" density="comfortable"
            :items="['California', 'Colorado', 'Florida', 'Georgia', 'Texas', 'Wyoming']"></v-select>
        </v-col>
        <v-col cols="7">
          <v-text-field clearable v-model="inputMessage" @keyup.enter="sendMessage" label="🗯️ Message" variant="underlined"
            required></v-text-field>
        </v-col>
        <v-col cols="2">
          <v-btn @click="sendMessage">
            <v-icon>mdi-send</v-icon>

          </v-btn>
        </v-col>
      </v-row>

    </v-bottom-navigation>
  </v-container>

</template>

<style scoped>
.fill-height {
  height: 80vh !important;
}

.fill-width {
  width: 80vw;
  overflow-x: hidden !important;
}


.request {
  /* background-color: #e0e0e0; */
  background: linear-gradient(135deg, rgba(224, 224, 224, 0.8), rgba(192, 192, 192, 0.8));
  align-self: flex-start;
  box-shadow: 0px 4px 6px rgba(0, 0, 0, 0.5);
}

.response {
  /* background-color: #a5d6a7; */
  background: linear-gradient(135deg, rgba(165, 214, 167, 0.8), rgba(129, 199, 132, 0.8));
  align-self: flex-end;
  box-shadow: 0px 4px 6px rgba(0, 0, 0, 0.5);
}

.pa-3 {
  padding: 16px !important;
}

.card-text {
  font-family: ui-sans-serif, -apple-system, system-ui, Segoe UI, Roboto, Ubuntu, Cantarell, Noto Sans, sans-serif, Helvetica, Apple Color Emoji, Arial, Segoe UI Emoji, Segoe UI Symbol !important;
  font-size: 1rem !important;
  font-variation-settings: normal !important;

}

.rounded-pill {
  border-radius: 15px !important;
  margin-left: 1vw;
}

.form {
  overflow-y: hidden !important;
}

.chat-area {
  overflow-y: auto;
  max-height: 75vh !important;
  font-family: ui-sans-serif, -apple-system, system-ui, Segoe UI, Roboto, Ubuntu, Cantarell, Noto Sans, sans-serif, Helvetica, Apple Color Emoji, Arial, Segoe UI Emoji, Segoe UI Symbol !important;
  font-size: 2rem !important;
}
</style>