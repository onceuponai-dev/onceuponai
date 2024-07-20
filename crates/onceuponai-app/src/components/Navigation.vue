<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from 'vue-router'

const drawer = ref(true);

const subtitle = ref("");

const title = ref("Once Upon ... AI");
const items = ref([
  { title: 'HOME', icon: 'mdi-home', route: '/' },
  { title: 'ACTORS', icon: 'mdi-puzzle', route: '/actors' },
  { title: 'CHAT', icon: 'mdi-chat', route: '/chat' },
  { title: 'DATASETS', icon: 'mdi-database', route: '/stores' },
  { title: 'EMBEDDINGS', icon: 'mdi-vector-triangle', route: '/embeddings' },
  { title: 'PROMPTS', icon: 'mdi-code-brackets', route: '/prompts' },
  { title: 'DASHBOARD', icon: 'mdi-view-dashboard', route: '/dashboard' },
  { title: 'PERSONAL TOKENS', icon: 'mdi-account-key', route: '/personal-tokens' },
  { title: 'SUPPORT', icon: 'mdi-help-circle', route: '/support' },
]);


const router = useRouter();
// router.push("/")
const navigate = (route: string) => {
  router.push(route);
};


</script>

<template>

  <v-navigation-drawer expand-on-hover rail permanent location="right" >
    <v-list>
      <v-list-item prepend-avatar="/images/logo100.png" :subtitle="subtitle" :title="title"></v-list-item>
    </v-list>
    <v-divider></v-divider>
    <v-list density="compact" nav>
      <v-list-item v-for="item in items" :key="item.title" @click="navigate(item.route)" :prepend-icon="item.icon"
        :title="item.title" link></v-list-item>
    </v-list>

  </v-navigation-drawer>


</template>
