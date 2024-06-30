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
