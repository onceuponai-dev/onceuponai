<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from 'vue-router'


const subtitle = ref("");

const title = ref("Once Upon ... AI");
const items = ref([
  // { title: 'HOME', icon: '$home', route: '/' },
  { title: 'ACTORS', icon: '$brain', route: '/actors' },
  { title: 'CHAT', icon: '$chat', route: '/chat' },
  { title: 'DATASETS', icon: '$database', route: '/stores' },
  { title: 'EMBEDDINGS', icon: '$embeddings', route: '/embeddings' },
  { title: 'PROMPTS', icon: '$prompts', route: '/prompts' },
  { title: 'DASHBOARD', icon: '$dashboard', route: '/dashboard' },
  { title: 'PERSONAL TOKENS', icon: '$tokens', route: '/personal-tokens' },
  { title: 'SUPPORT', icon: '$support', route: '/support' },
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
