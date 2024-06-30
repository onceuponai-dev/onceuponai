<script lang="ts">
import { defineComponent, ref, onMounted, onBeforeUnmount } from 'vue';
import axios from 'axios';
import { deleteCookie, getCookie, parseBool, setCookie } from './common';
import { useRouter } from 'vue-router'

export default defineComponent({
  name: 'App',
  components: {
  },
  setup() {

    const app: any = ref(null);

    const cookieConsentKey = "cookie-consent";
    const cookieConsentBanner = ref(true);

    const email: any = ref(null);
    const userName: any = ref(null);

    const drawer: any = ref(true);
    const items: any = ref([
      { title: 'ACTORS', icon: 'mdi-brain', route: '/actors' },
      { title: 'CHAT', icon: 'mdi-chat', route: '/chat' },
      { title: 'DATASETS', icon: 'mdi-database', route: '/stores' },
      { title: 'EMBEDDINGS', icon: 'mdi-vector-triangle', route: '/embeddings' },
      { title: 'PROMPTS', icon: 'mdi-code-brackets', route: '/prompts' },
      { title: 'DASHBOARD', icon: 'mdi-view-dashboard', route: '/dashboard' },
      { title: 'PERSONAL TOKENS', icon: 'mdi-account-key', route: '/personal-tokens' },
      { title: 'SUPPORT', icon: 'mdi-help-circle', route: '/support' },
    ]);

    const projects: any = ref([
      { name: 'Project 1' },
      { name: 'Project 2' },
      { name: 'Project 3' }
    ]);

    const router = useRouter();
    const navigate = (route: string) => {
      router.push(route);
    };

    const addProject = () => {
      // Logic to add a new project
      alert("Add new project logic here!");
    };


    axios.get(`/api/user`)
      .then(function (response) {
        email.value = response.data.email;
        userName.value = email.value.split("@")[0];
      })
      .catch(function (error) {
        console.log(error);
      });

    let redirectUrl = getCookie("redirectUrl");
    deleteCookie("redirectUrl");
    if (redirectUrl != null) {
      window.location.href = redirectUrl;
    }

    function acceptCookie() {
      setCookie(cookieConsentKey, "true", 365);
      cookieConsentBanner.value = false;
    }

    onMounted(() => {
      cookieConsentBanner.value = !parseBool(getCookie(cookieConsentKey));

    });

    onBeforeUnmount(() => {
    });


    return {
      cookieConsentBanner,
      acceptCookie,
      drawer,
      items,
      navigate,
      projects,
      addProject,
      email,
      userName
    };
  }
});
</script>

<template>
  <v-app>


    <v-navigation-drawer v-model="drawer" expand-on-hover rail>
      <v-list>
        <v-list-item prepend-avatar="/ui/images/logo100.png" :subtitle="email" :title="userName"></v-list-item>
      </v-list>
      <v-divider></v-divider>
      <v-list density="compact" nav>
        <v-list-item v-for="item in items" :key="item.title" @click="navigate(item.route)" :prepend-icon="item.icon"
          :title="item.title" link></v-list-item>
      </v-list>

    </v-navigation-drawer>

    <v-app-bar app>
      <v-toolbar-title>Once Upon ... AI</v-toolbar-title>

      <v-spacer></v-spacer>

      <v-menu offset-y>
        <template v-slot:activator="{ props }">
          <v-btn icon v-bind="props">
            <v-icon>mdi-dots-vertical</v-icon>
          </v-btn>
        </template>
        <v-list dense>
          <v-list-item v-for="(project, index) in projects" :key="index" class="menu-item">
            <v-list-item-content class="menu-item-content">
              <v-list-item-icon>
                <v-icon class="menu-icon">mdi-pencil</v-icon>
              </v-list-item-icon>
              <v-list-item-title>{{ project.name }}</v-list-item-title>
            </v-list-item-content>

          </v-list-item>
          <v-list-item @click="addProject" class="menu-item">

            <v-list-item-content class="menu-item-content">
              <v-list-item-icon>
                <v-icon class="menu-icon">mdi-plus</v-icon>
              </v-list-item-icon>
              <v-list-item-title>New Project</v-list-item-title>
            </v-list-item-content>
          </v-list-item>
        </v-list>
      </v-menu>
    </v-app-bar>


    <v-main>
      <router-view />
      <router-view name="footer" />
    </v-main>


    <!--

  <v-container>




    <div>
      <div class="rete" ref="rete"></div>
      <router-view />
      <router-view name="footer" />
    </div>
    <br />

    <br />
    <v-snackbar v-model="cookieConsentBanner" color="white">
      🍪 We use cookies to enhance your experience on our site.
      By clicking OK or continuing to use our site, you agree that these cookies can be placed.
      <template v-slot:actions>
        <v-btn color="blue" variant="text" @click="acceptCookie">
          OK
        </v-btn>
      </template>
    </v-snackbar>

  </v-container>
-->

  </v-app>

</template>

<style scoped>
.menu-item-content {
  display: flex;
  align-items: center;
}

.menu-icon {
  margin-right: 16px;
}
</style>
