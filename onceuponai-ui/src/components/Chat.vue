<script lang="ts">
import { defineComponent, ref, onMounted, nextTick } from 'vue';
import { useRouter } from 'vue-router';
import axios from 'axios';

interface Message {
  content: string;
  role: 'user' | 'assistant' | 'system';
}

interface Choice {
  message: Message
}

interface ChatResponse {
  choices: Choice[];
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
    const actors: any = ref([]);
    const selectedActor: any = ref(null);
    const isStream: any = ref(false);

    axios.get(`/api/actors`)
      .then(function (response) {
        var values = Object.keys(response.data).map((key) => {
          return response.data[key];
        }).filter((v) => v.metadata.features.includes("chat")).map((v) => `${v.kind}/${v.metadata.name}`);

        actors.value = values;
        if (values.length > 0) {
          selectedActor.value = values[0];
        }

      })
      .catch(function (error) {
        console.log(error);
      });

    const sendMessage = () => {
      if (inputMessage.value.trim() === '') return;

      var text = inputMessage.value;
      messages.value.push({ content: text, role: 'user' });
      inputMessage.value = '';
      showProgress.value = true;

      const controller = new AbortController();
      const signal = controller.signal;

      fetch(`/v1/chat/completions`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          stream: isStream.value,
          model: selectedActor.value,
          messages: [{ "content": text, "role": "user" }]
        }),
        signal: signal
      })
        .then(async response => {

          if (!response.ok) {
            throw new Error('Network response was not ok');
          }


          const reader = response.body?.getReader();
          const decoder = new TextDecoder('utf-8');
          let resultText = '';
          let done, value;
          let ix = 0;
          if (reader) {
            while ({ done, value } = await reader.read(), !done) {

              showProgress.value = false;
              let textChunk = decoder.decode(value, { stream: true });
              console.log(textChunk)
              resultText += textChunk;
              const messagesToSend = resultText.split('\n').filter(message => message.trim().length > 0);
              for (let message of messagesToSend) {
                try {
                  if (isStream.value) {
                    message = message.replaceAll("}{", "},{");
                    message = `[${message}]`;
                    let m: ChatResponse[] = JSON.parse(message);
                    let role = m[0].choices[0].message.role;
                    let content = m.map((x) => x.choices[0].message.content).join("");
                    if (ix === 0) {
                      messages.value.push({ "content": content, "role": role });
                    } else {
                      let lastMessage = messages.value[messages.value.length - 1];
                      lastMessage.content = content;
                    }
                  } else {
                    let m: ChatResponse = JSON.parse(message);
                    let choice = m.choices[0];
                    messages.value.push(choice.message);
                  }

                } catch (e) {
                  console.warn('Failed to parse message', e);
                }
              }

              nextTick(() => {
                setTimeout(() => {
                  var chatDiv = document.getElementsByClassName("chat-area")[0];
                  chatDiv.scrollTop = chatDiv.scrollHeight;
                }, 100);
              });
              ix++;
            }

            reader.releaseLock();
          }


        })
        .catch(error => {
          messages.value.push({ content: "😿 Error", role: 'assistant' });
          showProgress.value = false;
          console.log(error);
        });

      // Optional: Abort the request if needed
      // controller.abort();
    };












    // const sendMessage = () => {
    //   if (inputMessage.value.trim() === '') return;

    //   var text = inputMessage.value;
    //   messages.value.push({ content: text, role: 'user' });
    //   inputMessage.value = '';
    //   showProgress.value = true;
    //   axios.post(`/api/invoke/${selectedActor.value}`, {
    //     stream: isStream.value,
    //     config: {},
    //     data: {
    //       message: [{ "content": text, "role": "user" }],
    //     }
    //   }, {
    //     headers: {
    //       'Content-Type': 'application/json'
    //     }
    //   })
    //     .then(function (response) {

    //       showProgress.value = false;
    //       console.log(response);
    //       var result = response.data.results[0];
    //       messages.value.push({ content: result, role: 'assistant' });
    //       nextTick(() => {
    //         setTimeout(() => {
    //           var chatDiv = document.getElementsByClassName("chat-area")[0]
    //           chatDiv.scrollTop = chatDiv.scrollHeight;
    //         }, 100);
    //       });

    //     })
    //     .catch(function (error) {

    //       messages.value.push({ content: "😿 Error", role: 'assistant' });
    //       showProgress.value = false;
    //       console.log(error);
    //     });

    // };

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
      showProgress,
      selectedActor,
      actors,
      isStream
    };


  }
});
</script>

<template>
  <v-container class="d-flex flex-column fill-height">
    <v-card class="flex-grow-1 mb-2 overflow-auto fill-width chat-area" ref="chatArea">
      <v-row v-for="(message, index) in messages" :key="index" class="mb-2">
        <v-col :cols="message.role === 'user' ? '8' : '12'" :offset="message.role === 'user' ? '0' : '1'">
          <v-card :class="message.role" class="pa-3 rounded-pill" width="90%">
            <v-card-text class="card-text">{{ message.content }}</v-card-text>
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
          <v-select label="Actor" menu-icon="mdi-brain" bg-color="white" density="comfortable" v-model="selectedActor"
            :items="actors"></v-select>
        </v-col>
        <v-col cols="7">
          <v-text-field clearable v-model="inputMessage" @keyup.enter="sendMessage" label="🗯️ Message"
            variant="underlined" :disabled="actors == 0" required></v-text-field>
        </v-col>
        <v-col cols="1">
          <v-btn @click="sendMessage" :disabled="actors == 0">
            <v-icon>mdi-send</v-icon>

          </v-btn>
        </v-col>
        <v-col cols="1">
          <v-switch inset class="switch" v-model="isStream">
          </v-switch>
          <v-tooltip activator="parent" location="top">stream</v-tooltip>
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


.user {
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.9), rgba(220, 220, 220, 0.9));
  align-self: flex-start;
  box-shadow: 0px 4px 6px rgba(0, 0, 0, 0.5);
}

.assistant {
  background: linear-gradient(135deg, rgba(250, 250, 255, 0.9), rgba(220, 220, 225, 0.9));
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

.switch {
  margin-top: 7px;
}
</style>