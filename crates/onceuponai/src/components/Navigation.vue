<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from 'vue-router'
import { driver } from "driver.js";
import "driver.js/dist/driver.css";

const subtitle = ref("");

const driverObj = driver({
  showProgress: true,
  steps: [
    { popover: { title: 'Happy Coding', description: 'And that is all, go ahead and start adding tours to your applications.' } },
    { element: '#actors', popover: { title: 'Animated Tour Example', description: 'Here is the code example showing animated tour. Let\'s walk you through it.', side: "bottom", align: 'start' } },
    { element: '#chat', popover: { title: 'Import the Library', description: 'It works the same in vanilla JavaScript as well as frameworks.', side: "bottom", align: 'start' } },
    // { element: '#editor', popover: { title: 'Importing CSS', description: 'Import the CSS which gives you the default styling for popover and overlay.', side: "bottom", align: 'start' } },
    { element: '#personal-tokens', popover: { title: 'Importing CSS', description: 'Import the CSS which gives you the default styling for popover and overlay.', side: "bottom", align: 'start' } },
    { element: '#editorLeft', popover: { title: 'Importing CSS', description: 'Import the CSS which gives you the default styling for popover and overlay.', side: "bottom", align: 'start' } },
    { popover: { title: 'Happy Coding', description: 'And that is all, go ahead and start adding tours to your applications.' } }
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
