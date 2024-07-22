import { createRouter, createWebHashHistory } from 'vue-router';
import HomeComponent from './components/Home.vue';
import ActorsComponent from './components/Actors.vue';
import ChatComponent from './components/Chat.vue';
import FooterComponent from './components/Footer.vue';

const routes: Array<any> = [
  {
    path: '/',
    name: 'Home',
    components: {
      default: HomeComponent,
      footer: FooterComponent
    },
  },
  {
    path: '/actors',
    name: 'Actors',
    components: {
      default: ActorsComponent,
      footer: FooterComponent
    }
  },
  {
    path: '/chat',
    name: 'Chat',
    components: {
      default: ChatComponent,
      footer: FooterComponent
    }
  },


];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;