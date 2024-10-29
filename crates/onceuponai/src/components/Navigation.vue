<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from 'vue-router'
import { driver } from "driver.js";
import "driver.js/dist/driver.css";

const subtitle = ref("");

const driverObj = driver({
  showProgress: true,
  steps: [
    { popover: { title: "Welcome!", description: "Discover all the essential navigation options to make the most of our app!" } },
    { element: "#actors", popover: { title: "Actors", description: "Search, select, and spawn actors like LLM models to assist with tasks.", side: "bottom", align: 'start' } },
    { element: "#chat", popover: { title: "Chat", description: "Engage directly with your spawned LLMs for interactive conversations and support.", side: "bottom", align: 'start' } },
    // { element: "#editor", popover: { title: "Editor", description: "Use LLMs to tackle text tasks such as summarization and more."", side: "bottom", align: 'start' } },
    { element: "#personal-tokens", popover: { title: "Personal Tokens", description: "Generate tokens for easy access to API.", side: "bottom", align: 'start' } },
    // { element: "#editorLeft", popover: { title: "Editor", description: "Import the CSS which gives you the default styling for popover and overlay.", side: "bottom", align: 'start' } },
    { popover: { title: "Happy Prompting!", description: "You're all set to explore and create. Enjoy the journey!" } }
  ]
});

const title = ref("Once Upon ... AI");
const items = ref([
  // { title: 'HOME', icon: '$home', route: '/' },
  { id: "actors", title: 'ACTORS', icon: '$brain', route: '/actors' },
  { id: "chat", title: 'CHAT', icon: '$chat', route: '/chat' },
  // { title: 'DATASETS', icon: '$database', route: '/stores' },
  // { title: 'EMBEDDINGS', icon: '$embeddings', route: '/embeddings' },
  // { title: 'PROMPTS', icon: '$prompts', route: '/prompts' },
  // { title: 'DASHBOARD', icon: '$dashboard', route: '/dashboard' },

  // { id: "editor", title: 'EDITOR', icon: '$dashboard', route: '/editor' },
  { id: "personal-tokens", title: 'PERSONAL TOKENS', icon: '$tokens', route: '/personal-tokens' },
  // { id: "tour", title: 'TOUR', icon: '$support', route: '/support' },
]);


const router = useRouter();
// router.push("/")
const navigate = (route: string) => {
  router.push(route);
};


</script>

<template>

  <v-navigation-drawer expand-on-hover rail permanent location="right">
    <v-list>
      <v-list-item prepend-avatar="/images/logo100.png" :subtitle="subtitle" :title="title"></v-list-item>
    </v-list>
    <v-divider></v-divider>
    <v-list density="compact" nav>
      <v-list-item v-for="item in items" :id="item.id" :key="item.title" @click="navigate(item.route)"
        :prepend-icon="item.icon" :title="item.title" link></v-list-item>
      <v-list-item key="tour" @click="driverObj.drive()" prepend-icon="$support" title="TOUR" link></v-list-item>

    </v-list>

  </v-navigation-drawer>


</template>
