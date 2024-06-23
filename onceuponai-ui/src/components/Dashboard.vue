<script lang="ts">
import { defineComponent, defineEmits, ref, onMounted, onUpdated } from 'vue';
import { useRouter } from 'vue-router';
import { createEditor } from '../editor'

export default defineComponent({
  name: 'Dashboard',
  components: {
  },
  setup() {

    const rete: any = ref(null);
    onMounted(() => {

      createEditor(rete.value)

    });
    onUpdated(() => {});

    return {

      rete,
    };
  }
});
</script>

<template>
  <div class="rete" ref="rete"></div>
</template>

<style scoped>
body {
  margin: 0;
}
.rete {
  width: 90vw;
  height: 90vh;
}
</style>
