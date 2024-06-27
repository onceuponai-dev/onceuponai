import { createRouter, createWebHashHistory } from 'vue-router';
import HomeComponent from '@/components/Home.vue';
import PromptsComponent from '@/components/Prompts.vue';
import StoresComponent from '@/components/Stores.vue';
import TermsComponent from '@/components/Terms.vue';
import FooterComponent from '@/components/Footer.vue';
import ProfileComponent from '@/components/Profile.vue';
import ActorsComponent from '@/components/Actors.vue';
import DashboardComponent from '@/components/Dashboard.vue';
import ChatComponent from '@/components/Chat.vue';
import PersonalTokensComponent from '@/components/PersonalTokens.vue';

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
    path: '/profile',
    name: 'Profile',
    components: {
      default: ProfileComponent,
      footer: FooterComponent
    }
  },
  {
    path: '/prompts',
    name: 'Prompts',
    components: {
      default: PromptsComponent,
      footer: FooterComponent
    }
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
    path: '/terms',
    name: 'Terms',
    components: {
      default: TermsComponent,
      footer: FooterComponent
    }
  },
  {
    path: '/stores',
    name: 'Stores',
    components: {
      default: StoresComponent,
      footer: FooterComponent
    }
  },
  {
    path: '/dashboard',
    name: 'Dashboard',
    components: {
      default: DashboardComponent,
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
 {
    path: '/personal-tokens',
    name: 'PersonalTokens',
    components: {
      default: PersonalTokensComponent,
      footer: FooterComponent
    }
  },

];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;