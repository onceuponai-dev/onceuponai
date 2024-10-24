<script lang="ts">
import { defineComponent, ref, onMounted, nextTick } from 'vue';
import { useRouter } from 'vue-router';
import axios from 'axios';
import { parseMarkdown } from '../mdcommon';

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
    const isStream: any = ref(true);
    const dialog: any = ref(null);

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
          messages: messages.value
        }),
        signal: signal
      })
        .then(async (response: any) => {

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
      isStream,
      parseMarkdown,
      dialog
    };


  }
});
</script>

<template>
  <v-container fluid>
    <div class="flex-grow-1 mb-2 overflow-auto fill-width fill-height chat-area" ref="chatArea">
      <v-row v-for="(message, index) in messages" :key="index" class="mb-2">
        <v-col :cols="message.role === 'user' ? '8' : '10'" :offset="message.role === 'user' ? '0' : '1'">
          <div :class="message.role" class="pa-3 rounded-pill" width="90%">
            <v-divider :color="message.role == 'user' ? 'success' : 'info'">
              <v-chip class="mx-2 text-caption" :color="message.role == 'user' ? 'success' : 'info'">{{ message.role
                }}</v-chip>
            </v-divider>
            <div class="card-text" v-html="parseMarkdown(message.content)"></div>
          </div>
        </v-col>
        <v-divider v-if="message.role != 'user'" color="error"></v-divider>
      </v-row>
      <v-row>
        <v-col cols="4" offset="6" v-if="showProgress">
          <v-icon v-if="showProgress" icon="$brain" size="small" class="rotating"></v-icon>
        </v-col>
      </v-row>
    </div>
    <v-bottom-navigation color="primary" horizontal height="75">
      <v-row>
        <v-col cols="2" offset="1">
          <v-select label="Actor" menu-icon="$brain" bg-color="white" density="comfortable" v-model="selectedActor"
            :items="actors"></v-select>
        </v-col>
        <v-col cols="7">
          <v-text-field clearable v-model="inputMessage" @keyup.enter="sendMessage" label="Message" variant="underlined"
            :disabled="actors == 0" append-inner-icon="$openInNew" @click:append-inner="dialog = true"
            required></v-text-field>
        </v-col>
        <v-col cols="1">
          <v-btn @click="sendMessage" :disabled="actors == 0">
            <v-icon>$send</v-icon>

          </v-btn>
        </v-col>
        <v-col cols="1">
          <v-switch inset class="switch" v-model="isStream">
          </v-switch>
          <v-tooltip activator="parent" location="top">stream</v-tooltip>
        </v-col>

      </v-row>

    </v-bottom-navigation>
    <v-dialog v-model="dialog" width="90%">
      <v-card>
        <v-card-text>
          <v-textarea v-model="inputMessage" label="Message" rows="20"></v-textarea>
        </v-card-text>

        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="green darken-1" variant="text" @click="sendMessage">Send</v-btn>
          <v-btn color="gray darken-1" variant="text" @click="dialog = false">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-container>


</template>

<style scoped>
.fill-height {
  height: 80vh !important;
}

.fill-width {
  width: 95vw;
  overflow-x: hidden !important;
}

.user {
  align-self: flex-start;
}

.assistant {
  align-self: flex-end;
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
  font-family: ui-sans-serif, -apple-system, system-ui, Segoe UI, Roboto, Ubuntu, Cantarell, Noto Sans, sans-serif, Helvetica, Apple Color Emoji, Arial, Segoe UI Emoji, Segoe UI Symbol !important;
  font-size: 2rem !important;
}

.switch {
  margin-top: 7px;
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
</style>