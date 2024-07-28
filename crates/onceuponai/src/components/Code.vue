<script setup lang="ts">
import { ref, onMounted, onUpdated } from 'vue';
// import * as monaco from 'monaco-editor/esm/vs/editor/editor.api'
// @ts-ignore
// import * as python from 'monaco-editor/esm/vs/basic-languages/python/python'

import { asyncRun, formatCode, init_code, base_code } from '../pycommon';
import { parseMarkdown, highlight } from '../mdcommon';


const editor: any = ref(null);
const editorCreated: any = ref(false);
const code: any = ref(base_code);
const editing = ref(false);

const running: any = ref(false);
const loading = ref(true);

const results: any = ref("");
const error: any = ref("");
const done: any = ref(false);


const runCode = (code: any) => {
    loading.value = true;
    return asyncRun(code, {}).then((data: any) => {
        loading.value = false;
        return data;
    }).catch(err => {
        console.log(err);
    });
}

const runCell = (c: any) => {
    editing.value = false
    // let code_value = monaco.editor.getEditors()[0]?.getModel()?.getValue();
    let code_value = c;
    if (!code_value) {
        return;
    }

    let code = formatCode(code_value);
    running.value = true;
    asyncRun(code, {}).then((data: any) => {
        results.value = data.results;
        error.value = data.error;
        running.value = false;
        done.value = true;
    }).catch(err => {
        console.log(err);
        error.value = err;
        results.value = "";
        running.value = false;
        done.value = true;
    });
}

const runInitCode = (code: string) => {
    return runCode(code);
}


onMounted(() => {
    runCode(init_code);
    // if (!editorCreated.value) {
    //     editorCreated.value = true;
    //     monaco.languages.register({ id: "python" });
    //     monaco.languages.setMonarchTokensProvider('python', python.language);
    //     monaco.editor.create(editor.value, {
    //         value: "import pandas",
    //         language: 'python',
    //         // theme: "vs-dark",
    //         contextmenu: false
    //     })
    // }

});

onUpdated(() => {
});

</script>

<template>

    <v-container fluid>
        <div>
            <v-card-text>
                <v-container>
                    <v-row justify="center">
                        <v-col cols="1" sm="1" align="end">
                            <v-btn v-show="!running" @click="runCell(code)"
                                :icon="done ? '$check' : '$arrowRightDropCircleOutline'" size="small"></v-btn>
                            <v-progress-circular v-show="running" indeterminate color="blue"></v-progress-circular>
                        </v-col>

                        <v-col cols="11" sm="11">
                            <v-textarea spellcheck="false" v-if="editing" auto-grow v-model="code" rows="10"
                                class="code-container" label=""
                                v-on:keydown.ctrl.enter.capture.prevent.stop="runCell(code)"
                                v-on:blur="editing = false"></v-textarea>
                            <pre class="pre-container" v-if="!editing" v-on:click="editing = true"
                                v-html="highlight(code)"></pre>

                            <!-- <v-textarea v-model="code" label="Code Editor" rows="20" 
                                v-on:keydown.ctrl.enter.capture.prevent.stop="runCell(code)" outlined></v-textarea>-->
                            <!-- <div id="editor" ref="editor"></div> -->
                        </v-col>
                        <v-col cols="12" sm="12">
                            <div class="results-container" v-html="results"></div>
                            <v-card outlined color="red-lighten-3" v-if="error">
                                <v-card-title class="error--text">
                                    Error
                                </v-card-title>
                                <v-card-text class="error--text">{{ error }}
                                </v-card-text>
                            </v-card>
                        </v-col>
                    </v-row>
                </v-container>
            </v-card-text>
        </div>
    </v-container>
</template>

<style scoped>
#editor {
    /* width: 40dvw; */
    height: 200px;
}

.results-container {
    margin-left: 10%;
}

.code-container {
  font-family: monospace, monospace;
}

.pre-container {
  padding: 15px 16px;
  border-color: -internal-light-dark(rgb(118, 118, 118), rgb(133, 133, 133));
  background-color: #F6F6F6;
  font-family: monospace, monospace;
  margin-bottom: 22px;

  font-size: 16px;
}


.results-container {
  font-family: monospace, monospace;
}

.markdown-container {}

.toolbar-switch {
  margin-top: 20px;
  width: 45px;
  font-size: 5px;
}

.main-title {
    font-family: 'Fontdiner Swanky' !important;
    font-size: 11px !important;
    line-height: 1.5;
}
</style>