import { createApp } from "vue";
import App from "./App.vue";

import router from './router';

// Vuetify
import 'vuetify/styles'
import { aliases, mdi } from 'vuetify/iconsets/mdi-svg'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import { mdiHome, mdiBrain, mdiChat, mdiPuzzle, mdiDatabase, mdiVectorTriangle, mdiCodeBrackets, mdiViewDashboard, mdiAccountKey, mdiHelpCircle, mdiSend, mdiKeyPlus, mdiContentCopy, mdiDelete } from '@mdi/js'

const vuetify = createVuetify({
    components,
    directives,
    theme: {
        themes: {
            light: {
                dark: false,
                colors: {
                    primary: "#757575", // colors.red.darken1, // #E53935
                    secondary: "#F5F5F5", // colors.red.lighten4, // #FFCDD2
                }
            },
        },
    },
    icons: {
        defaultSet: 'mdi',
        aliases: {
            ...aliases,
            home: mdiHome,
            brain: mdiBrain,
            puzzle: mdiPuzzle,
            chat: mdiChat,
            database: mdiDatabase,
            embeddings: mdiVectorTriangle,
            prompts: mdiCodeBrackets,
            dashboard: mdiViewDashboard,
            tokens: mdiAccountKey,
            support: mdiHelpCircle,
            send: mdiSend,
            keyPlus: mdiKeyPlus,
            contentCopy: mdiContentCopy,
            delete: mdiDelete
        },
        sets: {
            mdi,
        },
    },
})



createApp(App).use(vuetify).use(router).mount('#app')