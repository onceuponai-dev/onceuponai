<script lang="ts">
import { defineComponent, defineEmits, ref, onMounted, onUpdated } from 'vue';
import { useRouter } from 'vue-router';

export default defineComponent({
  name: 'Footer',
  components: {
  },
  setup() {
    const router = useRouter();
    onMounted(() => { });
    onUpdated(() => { });

    return {
      router,
    };
  }
});
</script>

<template>
  <footer>
    <v-row v-if="router.currentRoute.value.name != 'Home'" justify="center" no-gutters>
      <router-link to="/">
        <img src="/images/logo100.png" />
      </router-link>
    </v-row><br />
    <v-row justify="center" no-gutters v-if="router.currentRoute.value.name != 'Home'" >
      <v-btn class="mx-2" rounded="xl" @click="router.push('/')">HOME</v-btn>
      <!-- <v-btn class="mx-2" rounded="xl" @click="router.push('/games')">GAMES</v-btn> -->
      <v-btn class="mx-2" rounded="xl" @click="router.push('/terms')">TERMS</v-btn>
    </v-row>
    <br/>
    <v-row justify="center" no-gutters>
      <v-col class="text-center mt-4" cols="12">
        <strong>© {{ new Date().getFullYear() }} <strong></strong>onceuponai.dev</strong>
      </v-col>
    </v-row>

  </footer>
</template>

<style scoped></style>
