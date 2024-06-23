<script lang="ts">
import { defineComponent, ref, onMounted } from 'vue';
import { useRouter } from 'vue-router';

export default defineComponent({
  name: 'Games',
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
        <v-card class="mx-auto" max-width="309">
          <v-img src="/intro/intro.png" cover></v-img>
          <v-card-title style="font-family: Bangers; font-size: 32px;">
            Data Driven Adventures
          </v-card-title>

          <v-card-subtitle>
          </v-card-subtitle>

          <v-card-actions>
            <v-btn color="blue-lighten-2" href="/games/datadrivenadventures/index.html" variant="outlined">
              Play
            </v-btn>
            <v-btn color="blue-lighten-2" variant="outlined" @click="showVideo('_rSK3io9Bbs')">
              Intro
            </v-btn>
            <v-spacer></v-spacer>
            <v-btn :icon="show ? 'mdi-chevron-up' : 'mdi-chevron-down'" @click="show = !show"></v-btn>
          </v-card-actions>

          <v-expand-transition>
            <div v-show="show">
              <v-divider></v-divider>
              <v-card-text>
                Embark on a thrilling journey in our immersive game set in a kingdom where magic and technology collide.
                Join our hero as they battle the malevolent wizard Floppy to reclaim stolen data and save the kingdom.
                This unique adventure not only entertains but also teaches data science using Python, Pandas,
                and other powerful libraries. Dive into the world of data while having a blast in this epic quest!
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

</style>