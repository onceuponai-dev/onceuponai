<script lang="ts">
import { defineComponent, defineEmits, ref, onMounted, onUpdated } from 'vue';
import { useRouter } from 'vue-router';

export default defineComponent({
  name: 'Home',

  props: {
    index: {
      type: Number,
      default: 0
    },
    card: {
      type: Array<any>,
      default: []
    }
  },
  emits: ['showcode'],
  setup(props, { emit }) {

    const router = useRouter();

    function showCode() {
      emit("showcode", props.index);
    }

    onMounted(() => {});

    onUpdated(() => {});

    return {
      showCode,
      router
    };
  }
});
</script>

<template>
    <v-container fluid fill-height class="home">
      <br/>
      <v-row align="center" justify="center">
        <v-col cols="auto">
          <v-img :width="200" src="/images/logo500.png" />
        </v-col>
      </v-row>
      <v-row align="center" justify="center">
        <v-col cols="auto">
          <v-btn variant="text" href="/cheatsheets/index.html" color="black" size="large" class="mx-2 swanky-font"
            rounded="lg" absolute>
            CHEAT SHEETS
          </v-btn>
        </v-col>
      </v-row>
      <v-row align="center" justify="center">
        <v-col cols="auto">
          <v-btn variant="text" @click="router.push('/stories')" color="black" size="large" class="mx-2 swanky-font"
            rounded="lg" absolute>
            STORIES
          </v-btn>
        </v-col>
      </v-row>

      <!-- <v-row align="center" justify="center">
        <v-col cols="auto">
          <v-btn variant="text" @click="router.push('/games')" color="black" size="large" class="mx-2 bangers-font"
            rounded="lg" absolute>
            GAMES
          </v-btn>
        </v-col>
      </v-row> -->
      <v-row align="center" justify="center">
        <v-col cols="auto">
          <v-btn variant="text" href="https://blog.qooba.net" color="black" size="large" class="mx-2 swanky-font"
            rounded="lg" absolute>
            BLOG
          </v-btn>
        </v-col>
      </v-row>
      <v-row align="center" justify="center">
        <v-col cols="auto">
          <v-btn variant="text" href="https://youtube.com/@qooba" size="large" class="mx-2 swanky-font" rounded="lg"
            absolute>
            <img src="/images/yt.png" />
          </v-btn>
        </v-col>
      </v-row>
      <br/>

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

.swanky-font {
  font-family: 'Fontdiner Swanky';
  font-size: 250%;
  padding: 7px;
  margin: 5px;
}

</style>
