<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { fetch } from "../common";
// import { parseMarkdown } from '../mdcommon';
import Quill from 'quill';
import "quill/dist/quill.snow.css";

interface Message {
  content: string;
  role: 'user' | 'assistant' | 'system';
}

const inputMessage = ref('');
const messages = ref<Message[]>([]);
const chatArea = ref<HTMLElement | null>(null);
const showProgress = ref(false);
const actors: any = ref([]);
const selectedActor: any = ref(null);
const isStream: any = ref(true);
const dialog: any = ref(null);

const editorContentLeft: any = ref(null);
const quillLeft: any = ref(null);
const quillRight: any = ref(null);
const quillEditorLeft: any = ref(null);
const quillEditorRight: any = ref(null);
const showChatInput: any = ref(false);
const selectedTool: any = ref(null);
const selectedPrompt: any = ref(null);
const selectedPromptText: any = ref("");
const versionIndex: any = ref(0);


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

    console.log(content);
    if (showProgress.value) {
      quillLeft.value.enable(false);
      quillRight.value.enable(true);
      quillLeft.value.root.classList.add('read-only');
      quillRight.value.root.classList.add('editable');
      quillRight.value.root.classList.remove('read-only');
      quillLeft.value.root.classList.remove('editable');

      messages.value.push({ "content": content, "role": role });
      showProgress.value = false;
      versionIndex.value = messages.value.length;
    } else {
      let lastMessage = messages.value[messages.value.length - 1];
      lastMessage.content += content;
    }

    quillRight.value.root.innerHTML = messages.value[messages.value.length - 1].content;
  }

});

const sendMessage = async () => {
  if (!showChatInput.value) {
    inputMessage.value = selectedPromptText.value.replace("{text}", quillLeft.value.root.innerHTML)
  }
  else if (inputMessage.value.trim() === '') {
    return;
  }

  var text = inputMessage.value;
  console.log(text)
  messages.value.push({ content: text, role: 'user' });
  inputMessage.value = '';
  showProgress.value = true;

  const config: any = await invoke("config");

  const message = {
    "stream": isStream.value,
    "model": selectedActor.value,
    "messages": [{ "content": text, "role": "user" }]
  };

  await invoke("v1_chat_completions", { "chatRequest": message, "baseUrl": config.base_url, "personalToken": config.personal_token });
};

// const getEditorContent = () => {
  // Fetch the editor's content (HTML format)
  // editorContent.value = quill.value.root.innerHTML;
  // console.log('Editor Content:', editorContent.value);

  // const range = quillLeft.value.getSelection();
  // console.log(range)
  // const selectedText = quill.value.getText(range.index, range.length);
  // editorContent.value = selectedText;
// };

const tools_db: any = [
  {
    "key": "Sumarize text",
    "prompts": [
      {
        "key": "Short Text",
        "prompt": `Jesteś doświadczonym językoznawcą. Twoim zadaniem jest upraszczanie tekstów tak aby były bardziej zrozumiałe dla ludzi.
----------
* Oto zasady prostego języka: Zasady prostego języka:
* Używaj krótkich zdań - każde zdanie powinno zawierać jedną myśl.
* Unikaj żargonu i skomplikowanych terminów - jeśli musisz użyć trudnych słów, wyjaśnij je prostym językiem.
* Używaj aktywnej formy czasowników - pisz, kto co robi, zamiast używać strony biernej.
* Podziel tekst na krótkie akapity - dłuższe teksty dziel na logiczne sekcje.
* Stosuj nagłówki i listy punktowane - ułatwiają one szybkie przyswajanie informacji.
* Używaj prostych słów - zamiast "rozpocząć", pisz "zacząć"; zamiast "zrealizować", pisz "wykonać".
* Pisz konkretnie - unikaj ogólników i nieprecyzyjnych informacji.
* Unikaj zbędnych słów - pisz zwięźle, skupiając się na sednie sprawy.
* Stosuj przykłady - jeśli opisujesz coś trudnego, daj przykład, by lepiej to zobrazować.
* Dostosuj język do odbiorcy - pamiętaj, kto będzie czytał tekst, i dostosuj do niego poziom trudności.
----------
Używają zasad prostego języka uprość poniższy tekst:

{text}
`
      }
    ]
  }
];

const fetchTools = (tools: any) => {
  return tools.map((tool: any) => tool.key);
};

const fetchPrompts = (tools: any, toolKey: string) => {
  return tools.filter((tool: any) => tool.key == toolKey)[0].prompts.map((tool: any) => tool.key);
};

const fetchPromptText = (tools: any, toolKey: string, promptKey: string) => {
  return tools.filter((tool: any) => tool.key == toolKey)[0].prompts.filter((prompt: any) => prompt.key == promptKey)[0].prompt;
};



const tools = fetchTools(tools_db);
selectedTool.value = tools[0];
const prompts = fetchPrompts(tools_db, selectedTool.value);
selectedPrompt.value = prompts[0];
selectedPromptText.value = fetchPromptText(tools_db, selectedTool.value, selectedPrompt.value);


onMounted(() => {
  if (chatArea.value) {
    chatArea.value.scrollTop = chatArea.value.scrollHeight;
  }

  quillLeft.value = new Quill(quillEditorLeft.value, {
    theme: 'snow',
  });

  quillRight.value = new Quill(quillEditorRight.value, {
    theme: 'snow',
  });

  quillRight.value.enable(false);
  quillRight.value.root.classList.add('read-only');
  quillLeft.value.root.classList.add('editable');
  //quill2.root.classList.remove('read-only');


  quillLeft.value.on('text-change', () => {

    editorContentLeft.value = quillLeft.value.root.innerHTML;
    quillRight.value.root.innerHTML = quillLeft.value.root.innerHTML;
  });

  // quillLeft.value.on('selection-change', (range: any, oldRange: any, source: any) => {
  //   if (range) {
  //     console.log('New selection:', range);
  //   } else {
  //     console.log('Selection cleared');
  //   }
  // });

});

// const fetchVersionContent = () => {
// }

</script>

<template>
  <v-container fluid class="container">
    <v-row>
      <v-col cols="6">
          <v-btn color="gray darken-1" variant="text" @click="dialog = false"><v-icon>$chevronLeft</v-icon></v-btn>
            page: 1/{{ messages.length }}
          <v-btn color="gray darken-1" variant="text" @click="dialog = false"><v-icon>$chevronRight</v-icon></v-btn>
      </v-col>

      <v-col cols="6">
          <v-btn color="gray darken-1" variant="text" @click="dialog = false"><v-icon>$chevronLeft</v-icon></v-btn>
            version: {{ versionIndex }}/{{ messages.length }}
          <v-btn color="gray darken-1" variant="text" @click="dialog = false"><v-icon>$chevronRight</v-icon></v-btn>
      </v-col>
    </v-row>
    <v-spacer></v-spacer>

    <v-row class="editors-row">
      <v-col cols="6">
        <div id="editorLeft" ref="quillEditorLeft">
Realizacja procesu wdrażania nowoczesnych technologii informatycznych w przedsiębiorstwie wymaga dokładnego zrozumienia szeregu skomplikowanych aspektów technicznych, organizacyjnych oraz operacyjnych. W związku z tym, niezbędne jest zapewnienie odpowiedniego przeszkolenia kadry zarządzającej oraz szeregowych pracowników, celem zwiększenia efektywności i minimalizacji ryzyka niepowodzeń.
        </div>
      </v-col>
      <v-col cols="6">
        <div id="editorRight" ref="quillEditorRight">
        </div>
      </v-col>

    </v-row>

    <!--
    <v-btn @click="getEditorContent">
      Get Editor Content
    </v-btn>

    <v-card>
      <v-card-title>Editor Content</v-card-title>
      <v-card-text>
        <pre>{{ editorContent }}</pre>
      </v-card-text>
    </v-card>
  -->
    <v-bottom-navigation color="primary" horizontal height="75" v-if="!showChatInput">
      <v-row>
        <v-col cols="3" class="actors-select">
          <v-select label="Actor" menu-icon="$brain" bg-color="white" density="comfortable" v-model="selectedActor"
            :items="actors" class="bottom-select"></v-select>
        </v-col>
        <v-col cols="3">
          <v-select label="Tool" menu-icon="$tools" bg-color="white" density="comfortable" v-model="selectedTool"
            :items="tools" class="bottom-select"></v-select>
        </v-col>
        <v-col cols="3">
          <v-select label="Prompt" menu-icon="$textBoxOutline" bg-color="white" density="comfortable"
            v-model="selectedPrompt" :items="prompts" class="bottom-select"></v-select>
        </v-col>
        <v-col cols="1">
          <v-btn @click="dialog = true" :disabled="actors == 0">
            <v-icon>$textBoxEditOutline</v-icon>
          </v-btn>
        </v-col>

        <v-col cols="1">
          <v-btn @click="sendMessage" :disabled="actors == 0">
            <v-icon>$send</v-icon>
          </v-btn>
        </v-col>
      </v-row>
    </v-bottom-navigation>

    <v-bottom-navigation color="primary" horizontal height="75" v-if="showChatInput">
      <v-row>
        <v-col cols="2" class="actors-select">
          <v-select label="Actor" menu-icon="$brain" bg-color="white" density="comfortable" v-model="selectedActor"
            :items="actors" class="bottom-select"></v-select>
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
      </v-row>
    </v-bottom-navigation>


    <v-dialog v-model="dialog" width="90%">
      <v-card>
        <v-card-text>
          <v-textarea v-model="selectedPromptText" label="Prompt" rows="15"></v-textarea>
        </v-card-text>

        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="green darken-1" variant="elevated" @click="sendMessage">Send</v-btn>
          <v-btn color="gray darken-1" variant="text" @click="dialog = false">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>


  </v-container>


</template>

<style scoped>
.container {
  height: 89%;
  padding-top: 1vh;
}

.editors-row {
  height: 100%;
}


.actors-select {
  margin-left: 2%;
}

.bottom-select {
  margin-top: 5%;
}
</style>