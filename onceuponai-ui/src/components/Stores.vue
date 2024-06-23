<script lang="ts">
import { defineComponent, defineEmits, ref, onMounted, onUpdated } from 'vue';
import { useRouter } from 'vue-router';

export default defineComponent({
  name: 'About',
  components: {
  },
  setup() {
    onMounted(() => {});
    onUpdated(() => {});

    return {
    };
  }
});
</script>

<template>
<v-speed-dial
  location="bottom right"
  transition="fade-transition"
  class="b1"
>
  <template v-slot:activator="{ props: activatorProps }">
    <v-fab
      location="start" 
      v-bind="activatorProps"
      color="success"
      extended
      prepend-icon="$vuetify"
      text="Extended"
    ></v-fab>
  </template>

  <v-btn key="1" prepend-icon="$success">T1dsfdsfds</v-btn>
  <v-btn key="2" prepend-icon="$info">T2</v-btn>
  <v-btn key="3" prepend-icon="$warning">T3</v-btn>
  <v-btn key="4" prepend-icon="$error">T4</v-btn>
</v-speed-dial>

<v-speed-dial
  location="bottom right"
  transition="fade-transition"
  class="b2"
>
  <template v-slot:activator="{ props: activatorProps }">
    <v-fab
    
      v-bind="activatorProps"
      color="success"
      extended
      prepend-icon="$vuetify"
      text="Extended"
    ></v-fab>
  </template>

  <v-btn key="1" prepend-icon="$success">T1dsfdsfds</v-btn>
  <v-btn key="2" prepend-icon="$info">T2</v-btn>
  <v-btn key="3" prepend-icon="$warning">T3</v-btn>
  <v-btn key="4" prepend-icon="$error">T4</v-btn>
</v-speed-dial>
</template>


<style scoped>
  .b2 {
    margin-left: 100px;
  }
</style>
