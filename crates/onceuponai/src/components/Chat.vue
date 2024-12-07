<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { fetch } from "../common";
import { parseMarkdown } from '../mdcommon';

interface Message {
  content: string;
  role: 'user' | 'assistant' | 'system';
}

const inputMessage = ref<string>('');
const messages = ref<Message[]>([]);
const chatArea = ref<HTMLElement | null>(null);
const showProgress = ref<boolean>(false);
const actors: any = ref([]);
const selectedActor: any = ref(null);
const isStream = ref<boolean>(true);
const dialog: any = ref<boolean | null>(null);
const temperature = ref<number>(1.0);
const seed = ref<number | null>(299792458);
const topP = ref<number>(1.0);
const system = ref<string>('');

fetch(`/api/actors`)
  .then(async (response: any) => {
    const data = await response.json();
    var values = Object.keys(data).map((key) => {
      return data[key];
    }).filter((v) => v.metadata.features.includes("chat")).map((v) => `${v.kind}/${v.metadata.name}`);

    actors.value = values;
    if (values.length > 0) {
      selectedActor.value = values[0];
    }

  })
  .catch(function (error: any) {
    console.log(error);
  });

listen('v1-chat-completions', event => {
  if (event != undefined) {
    let payload: any = event.payload;
    let m: any = JSON.parse(payload);
    let role = m.choices[0].message.role;
    let content = m.choices[0].message.content;

    if (showProgress.value) {
      messages.value.push({ "content": content, "role": role });
      showProgress.value = false;
    } else {
      let lastMessage = messages.value[messages.value.length - 1];
      console.log(lastMessage);
      lastMessage.content += content;
    }
  }

});

const sendMessage = async () => {
  if (inputMessage.value.trim() === '') return;

  var text = inputMessage.value;
  if (messages.value.length == 0 && system.value != '') {
    messages.value.push({content: system.value, role: 'system'})
  }

  messages.value.push({ content: text, role: 'user' });
  inputMessage.value = '';
  showProgress.value = true;

  const config: any = await invoke("config");
  console.log(JSON.stringify(messages.value));

  const message = {
    "stream": isStream.value,
    "model": selectedActor.value,
    "messages": messages.value,
    "seed": seed.value,
    "top_p": topP.value,
    "temperature": temperature.value
  };

  await invoke("v1_chat_completions", { "chatRequest": message, "baseUrl": config.base_url, "personalToken": config.personal_token });
};

onMounted(() => {
  if (chatArea.value) {
    chatArea.value.scrollTop = chatArea.value.scrollHeight;
  }
});

</script>

<template>
  <v-container fluid>
    <div class="flex-grow-1 mb-2 overflow-auto fill-width chat-area" ref="chatArea">
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
        <v-col cols="2" class="actors-select">
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
          <v-textarea v-model="inputMessage" label="Message" rows="15"></v-textarea>
        </v-card-text>

        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="green darken-1" variant="elevated" @click="sendMessage">Send</v-btn>
          <v-btn color="gray darken-1" variant="text" @click="dialog = false">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-container>

  <v-navigation-drawer expand-on-hover rail permanent location="left">
    <v-divider></v-divider>
    <br />
    <v-list density="compact" nav>
      <v-list-item id="temperature" key="temperature" prepend-icon="$thermometer" title="Temperature">
        <v-slider v-model="temperature" :max="1" :min="0" :step="0.1" width="80%" thumb-size="10"
          thumb-label></v-slider>
      </v-list-item>
      <v-list-item id="top_p" key="top_p" prepend-icon="$topP" title="Top_p">
        <v-slider v-model="topP" :max="1" :min="0" :step="0.1" width="80%" thumb-size="10" thumb-label></v-slider>
      </v-list-item>
      <v-list-item id="seed" key="seed" prepend-icon="$seed" title="Seed">
        <v-text-field v-model="seed" type="number" outlined />
      </v-list-item>
      <v-list-item id="system" key="system" prepend-icon="$robot" title="System">
        <v-textarea v-model="system" rows="3"></v-textarea>
      </v-list-item>
      <v-list-item prepend-icon="$cog" title="More" link></v-list-item>



    </v-list>

  </v-navigation-drawer>



</template>

<style scoped>
.fill-height {
  height: 80vh !important;
}

.fill-width {
  width: 90vw;
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

.actors-select {
  margin-left: 2%;
}
</style>