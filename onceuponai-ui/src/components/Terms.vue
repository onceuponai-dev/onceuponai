<script lang="ts">
import { defineComponent, defineEmits, ref, onMounted, onUpdated } from 'vue';
import { useRouter } from 'vue-router';

export default defineComponent({
  name: 'About',
  components: {
  },
  setup() {
    const router = useRouter();
    onMounted(() => { });
    onUpdated(() => { });

    return {
      router
    };
  }
});
</script>

<template>
  <v-container>
    <h1>Terms of Use</h1>
    <br/>
    <h2>1. Acceptance of Terms</h2>
    By accessing and using OnceUponAI, you agree to be bound by these Terms of Use and all applicable laws and
    regulations. If you do not agree with any part of these terms, you are prohibited from using or accessing this site.

    <br/><br/>
    <h2>2. User Eligibility</h2>
    This website is not intended for use by persons under the age of 13. By using OnceUponAI, you represent and
    warrant that you are of legal age to form a binding contract.

    <br/><br/>
    <h2>3. Use License</h2>
    Permission is granted to temporarily download one copy of the materials (information or software) on OnceUponAI for
    personal, non-commercial transitory viewing only.
    This is the grant of a license, not a transfer of title, and under this license, you may not:
    modify or copy the materials;
    use the materials for any commercial purpose, or for any public display (commercial or non-commercial);
    attempt to decompile or reverse engineer any software contained on OnceUponAI;
    transfer the materials to another person or "mirror" the materials on any other server.

    <br/><br/>
    <h2>4. User-Generated Content</h2>
    Users may post and exchange opinions, comments, and other content as long as the content is not illegal, obscene,
    threatening, defamatory, infringing on intellectual property rights, or otherwise injurious to third parties.

    <br/><br/>
    <h2>5. Disclaimer</h2>
    The materials on OnceUponAI are provided on an 'as is' basis. OnceUponAI makes no warranties, expressed or implied,
    and hereby disclaims and negates all other warranties including, without limitation, implied warranties or conditions
    of merchantability, fitness for a particular purpose, or non-infringement of intellectual property or other violation
    of rights.

    <br/><br/>

    <h2>6. Limitations</h2>
    In no event shall OnceUponAI or its suppliers be liable for any damages (including, without limitation, damages for
    loss of data or profit, or due to business interruption) arising out of the use or inability to use the materials on
    OnceUponAI.

    <br/><br/>
    <h2>7. Code and Applications</h2>
    Use of Code and Applications: OnceUponAI may provide various code samples, applications, and other software tools
    (hereafter referred to as "Software"). This Software is provided "as is" and is intended for educational and
    informational purposes only.
    No Warranty: OnceUponAI makes no warranties, express or implied, regarding the performance, functionality, or
    reliability of the Software. The user assumes all responsibility for the selection, installation, use, and results
    obtained from the Software.
    Limitation of Liability: In no event will OnceUponAI, its owners, employees, or affiliates be liable for any direct,
    indirect, incidental, special, exemplary, or consequential damages (including but not limited to procurement of
    substitute goods or services; loss of use, data, or profits; or business interruption) however caused and on any
    theory of liability, whether in contract, strict liability, or tort (including negligence or otherwise) arising in any
    way out of the use of this Software, even if advised of the possibility of such damage.
    No Guarantee: There is no guarantee that the Software will be free of defects, errors, or harmful components. Users
    are advised to exercise caution and due diligence in using the Software and to employ appropriate measures to
    safeguard their systems, such as using antivirus software and creating backups.

    <br/><br/>
    <h2>8. Accuracy of Materials</h2>
    The materials appearing on OnceUponAI could include technical, typographical, or photographic errors. OnceUponAI does
    not warrant that any of the materials on its website are accurate, complete, or current.

    <br/><br/>
    <h2>9. Links</h2>
    OnceUponAI has not reviewed all of the sites linked to its website and is not responsible for the contents of any such
    linked site.

    <br/><br/>
    <h2>10. Modifications</h2>
    OnceUponAI may revise these terms of use for its website at any time without notice.

    <br/><br/>
    <h2>11. Governing Law</h2>
    These terms and conditions are governed by and construed in accordance with the laws of Poland and you
    irrevocably submit to the exclusive jurisdiction of the courts in that location.

    <br/><br/>
    <h2>12. Contact Information</h2>
    For any questions regarding these Terms of Use, please contact us at qooba@onceuponai.dev.

</v-container></template>

<style scoped></style>
