<script lang="ts">
import { defineComponent, ref, onMounted } from 'vue';
import { useRouter } from 'vue-router';

export default defineComponent({
  name: 'Stories',
  components: {
  },
  setup() {
    const videoDialog = ref(false);
    const show = ref(false);
    const videoSource = ref("");
    const router = useRouter();

    onMounted(() => { });

    const showVideo = (video: string) => {
      videoSource.value = `https://www.youtube.com/embed/${video}`;
      videoDialog.value = true;
    };

    return {
      videoDialog,
      show,
      showVideo,
      videoSource,
      router
    };

  }
});
</script>

<template>
  <v-container>
    <br />
    <v-row dense align="center" justify="center">
      <v-col cols="auto">
        <v-card class="mx-auto" width="309">
          <v-img src="/images/LLMCheatsheets.jpeg" cover></v-img>
          <v-card-title class="swanky-font">
          </v-card-title>

          <v-card-subtitle>
          </v-card-subtitle>

          <v-card-actions>
            <v-btn color="blue-lighten-2" href="/cheatsheets/index.html" variant="outlined">
              Check
            </v-btn>
            <v-btn color="blue-lighten-2" variant="outlined" @click="showVideo('RDhXRGfeCwk')">
              Intro
            </v-btn>
            <v-spacer></v-spacer>
            <v-btn :icon="show ? 'mdi-chevron-up' : 'mdi-chevron-down'" @click="show = !show"></v-btn>
          </v-card-actions>

          <v-expand-transition>
            <div v-show="show">
              <v-divider></v-divider>
              <v-card-text>
                🧞‍♂️ Rub a lamp and unleash the LLM genie from interactive cheat sheets! Get your coding wishes granted
                with LLM-powered insights – all in your browser.
              </v-card-text>
            </div>
          </v-expand-transition>
        </v-card>
      </v-col>
    </v-row>
    <v-row dense align="center" justify="center">
      <v-col cols="auto">
        <v-card class="mx-auto" width="309">
          <v-img src="/images/ThumbeLLama.jpeg" cover></v-img>
          <v-card-title class="swanky-font">
          </v-card-title>

          <v-card-subtitle>
          </v-card-subtitle>

          <v-card-actions>
            <v-btn color="blue-lighten-2" href="https://stories.onceuponai.dev/stories-thumbellama/" variant="outlined">
              Check
            </v-btn>
            <v-btn color="blue-lighten-2" variant="outlined" @click="showVideo('Cex6d4ZnmhY')">
              Intro
            </v-btn>
            <v-spacer></v-spacer>
            <v-btn :icon="show ? 'mdi-chevron-up' : 'mdi-chevron-down'" @click="show = !show"></v-btn>
          </v-card-actions>

          <v-expand-transition>
            <div v-show="show">
              <v-divider></v-divider>
              <v-card-text>
                Story about TinyLLama, a 1.1 billion parameter language model. 
                How to convert it to WebAssembly using mlc-llm and run on typescript web application! 
              </v-card-text>
            </div>
          </v-expand-transition>
        </v-card>
      </v-col>
    </v-row>

    <br />
    <v-dialog v-model="videoDialog" max-width="900">
      <v-card>
        <v-card-text>
          <div class="video-container">
            <iframe width="900" :src="videoSource" frameborder="0" allowfullscreen></iframe>
          </div>
        </v-card-text>
      </v-card>
    </v-dialog>

    <br /><br /><br />
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