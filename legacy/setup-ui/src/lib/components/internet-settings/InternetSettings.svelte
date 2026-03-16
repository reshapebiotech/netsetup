<script lang="ts">
  import { form, field, combined } from 'svelte-forms';
  import { pattern } from 'svelte-forms/validators';

  import { dnsMethod, networkConfig } from '$lib/stores';
  import Button from '../button/Button.svelte';
  import Select from '../select/Select.svelte';
  import Input from '../input/Input.svelte';
  import { Minus, Plus } from 'phosphor-svelte';

  export let handleSubmit: (e: Event) => void;
  export let goBack: () => void;
  const sectionStyle = 'flex items-center gap-4 justify-between';
  const inputParentStyle = 'flex-1 xs:max-w-[312px] max-w-[312px]';

  const ipAddress = field(
    'IPAddress',
    $networkConfig.ipv4_address,
    [pattern(/^((25[0-5]|(2[0-4]|1\d|[1-9]|)\d)\.?\b){4}$/)],
    {
      valid: false,
      validateOnChange: true,
    }
  );
  const gateway = field(
    'Gateway',
    $networkConfig.ipv4_gateway,
    [pattern(/^((25[0-5]|(2[0-4]|1\d|[1-9]|)\d)\.?\b){4}$/)],
    {
      valid: false,
    }
  );
  const createDNS = (values: string[]) =>
    values.map((v, index) =>
      field(`DNS${index}`, v, [pattern(/^((25[0-5]|(2[0-4]|1\d|[1-9]|)\d)\.?\b){4}$/)], {
        valid: false,
        validateOnChange: true,
        checkOnInit: true,
      })
    );

  let dnsFields: string[] = [''];

  const addDNS = () => {
    dnsFields = [...dnsFields, ''];
  };

  $: dnsForm = combined('DNS', createDNS(dnsFields), (fields) =>
    fields.map((f: any) => f.value).join(',')
  );

  const removeDNS = () => {
    if (dnsFields.length === 1) return;
    dnsFields = dnsFields.slice(0, -1);
  };

  const ipValues = form(ipAddress, gateway);

  $: if ($networkConfig.ipv4_method !== 'manual') {
    ipAddress.set('');
    $networkConfig.ipv4_address = '';
    gateway.set('');
    $networkConfig.ipv4_gateway = '';
  }

  $: if ($dnsMethod !== 'manual') {
    dnsFields = [''];
  }

  $: $networkConfig.ipv4_address = $ipAddress.value;
  $: $networkConfig.ipv4_gateway = $gateway.value;
  $: $networkConfig.dns = dnsFields.filter((dns) => dns !== '');
</script>

<div class="flex flex-col gap-2">
  <form on:submit={handleSubmit} class="flex flex-col gap-4">
    <div class="flex flex-col gap-8 bg-white-500 rounded-xl shadow p-5">
      <div>
        <h5 class="pb-3">TCP/IP</h5>
        <div class="flex text-nowrap flex-col gap-3">
          <section class={sectionStyle}>
            <p>Configure IPv4</p>
            <div class={inputParentStyle}>
              <Select name="method" id="method" bind:value={$networkConfig.ipv4_method}>
                <option selected value="auto">Automatic (DHCP)</option>
                <option value="manual">Manual</option>
              </Select>
            </div>
          </section>
          {#if $networkConfig.ipv4_method === 'manual'}
            <section class={sectionStyle}>
              <p class="whitespace-nowrap">IP address</p>
              <div class={inputParentStyle}>
                <Input name="IPAddress" placeholder="IP Adress" bind:value={$ipAddress.value} />
              </div>
            </section>
            <section class={sectionStyle}>
              <p class="whitespace-nowrap">Prefix</p>
              <div class={inputParentStyle}>
                <Select name="Prefix" id="Prefix" bind:value={$networkConfig.ipv4_prefix}>
                  <option value="30">255.255.255.252 (/30 - 2 hosts)</option>
                  <option value="29">255.255.255.248 (/29 - 6 hosts)</option>
                  <option value="28">255.255.255.240 (/28 - 14 hosts)</option>
                  <option value="27">255.255.255.224 (/27 - 30 hosts)</option>
                  <option value="26">255.255.255.192 (/26 - 62 hosts)</option>
                  <option value="25">255.255.255.128 (/25 - 126 hosts)</option>
                  <option selected value="24">255.255.255.0 (/24 - 254 hosts)</option>
                  <option value="23">255.255.254.0 (/23 - 510 hosts)</option>
                  <option value="22">255.255.252.0 (/22 - 1022 hosts)</option>
                  <option value="21">255.255.248.0 (/21 - 2046 hosts)</option>
                  <option value="20">255.255.240.0 (/20 - 4094 hosts)</option>
                  <option value="16">255.255.0.0 (/16 - 65534 hosts)</option>
                  <option value="8">255.0.0.0 (/8 - 16777214 hosts)</option>
                </Select>
              </div>
            </section>
            <section class={sectionStyle}>
              <p class="whitespace-nowrap">Gateway</p>
              <div class={inputParentStyle}>
                <Input name="Gateway" placeholder="Gateway" bind:value={$gateway.value} />
              </div>
            </section>
          {/if}
        </div>
      </div>
      <div>
        <h5 class="pb-3">DNS</h5>
        <div class="flex flex-col gap-3">
          <section class={sectionStyle}>
            <p>DNS method</p>
            <div class={inputParentStyle}>
              <Select name="DNSMethod" id="DNSMethod" bind:value={$dnsMethod}>
                <option selected value="auto">Automatic</option>
                <option value="manual">Manual</option>
              </Select>
            </div>
          </section>
          {#if $dnsMethod === 'manual'}
            <section class="flex gap-4 justify-between">
              <p class="pt-4">DNS</p>
              <div class={inputParentStyle}>
                <div class="flex flex-col gap-1">
                  {#each dnsFields as dns, index}
                    <div class="flex flex-row gap-0.5 items-center">
                      <Input name={`DNS ${index + 1}`} placeholder="DNS" bind:value={dns} />
                      <Button iconButton size="xs" on:click={() => addDNS()}>
                        <Plus />
                      </Button>
                      <Button iconButton size="xs" on:click={() => removeDNS()}>
                        <Minus />
                      </Button>
                    </div>
                  {/each}
                </div>
              </div>
            </section>
          {/if}
        </div>
      </div>
    </div>
    <div class="flex flex-col-reverse sm:flex-row sm:justify-end gap-2">
      <Button variant="btn-outline btn-primary" on:click={goBack}>Previous</Button>
      <Button
        disabled={($networkConfig.ipv4_method === 'manual' && !$ipValues.valid) ||
          ($dnsMethod === 'manual' && !$dnsForm.valid)}
        variant="btn-primary"
        type="submit">Continue</Button
      >
    </div>
  </form>
</div>
