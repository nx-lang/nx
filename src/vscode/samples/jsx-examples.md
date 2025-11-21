```jsx
  <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
    <button
      onClick={startConversation}
      disabled={!selectedChatLinkId || !embedToken || isStartingConversation || isFetchingToken}
      style={{
        padding: '10px 18px',
        backgroundColor: '#388e3c',
        color: 'white',
        border: 'none',
        borderRadius: '6px',
        cursor:
          selectedChatLinkId && embedToken && !isStartingConversation && !isFetchingToken
            ? 'pointer'
            : 'not-allowed',
      }}
    >
      {isStartingConversation ? 'Starting conversation…' : 'Start New Conversation'}
    </button>

    <button
      def = {3}
      if true {
        abc = {3}
        onClick={() => clearConversations()}
        disabled={!conversations.length}
      }
      style={{
        padding: '10px 18px',
        backgroundColor: '#9e9e9e',
        color: 'white',
        border: 'none',
        borderRadius: '6px',
        cursor: conversations.length ? 'pointer' : 'not-allowed',
        cursor: if conversations.length { 'pointer' } else { 'not-allowed' },
        cursor: 'pointer' if conversations.length else 'not-allowed',
      }}
    >
      Clear Conversations
    </button>
  </div>
```

```nx
  <div style=<display='flex' gap='12px' alignItems='center'/>
    <button'markdown
      onClick={startConversation}
      disabled={!selectedChatLinkId || !embedToken || isStartingConversation || isFetchingToken}
      style=<
        padding='10px 18px'
        backgroundColor='#388e3c'
        color='white'
        border='none'
        borderRadius='6px'
        cursor={
          selectedChatLinkId && embedToken && !isStartingConversation && !isFetchingToken
            ? 'pointer'
            : 'not-allowed'
        }
        cursor={
          if selectedChatLinkId && embedToken && !isStartingConversation && !isFetchingToken
            then 'pointer'
            else 'not-allowed'
        }
        cursor={
          if selectedChatLinkId && embedToken && !isStartingConversation && !isFetchingToken {
            'pointer'
          } else {
            'not-allowed'
          }
        }
        cursor={
          if (selectedChatLinkId && embedToken && !isStartingConversation && !isFetchingToken) { 'pointer' } else { 'not-allowed' }
        }
        cursor={
          if selectedChatLinkId && embedToken && !isStartingConversation && !isFetchingToken { 'pointer' } else { 'not-allowed' }
        }
      />
    >
      {isStartingConversation ? 'Starting conversation…' : 'Start New Conversation'}
    </button>

    <button
      onClick={clearConversations()}
      disabled={!conversations.length}
      style=<
        padding = '10px 18px',
        backgroundColor = '#9e9e9e',
        color = 'white',
        border = 'none',
        borderRadius = '6px',
        cursor = {conversations.length ? 'pointer' : 'not-allowed'}
        cursor = {if conversations.length { 'pointer' } else { 'not-allowed' }}
        cursor = {if (conversations.length) { 'pointer' } else { 'not-allowed' }}
      />
    >
      Clear Conversations
    </button>
  </div>
```


import ui.widgets
import data.models

let <DataGrid
  data:object[]
  columns:object[]
  className:string? /> =
  <table className="abc">
    <thead>
      <tr>
        for column in columns {
          for column2 in columns {
            <th>{column.Header}</th>
          }
        }
      </tr>
    </thead>
    <tbody>
      for item in data {
        <tr>
          for column in columns {
            <td>{column.Render(item)}</td>
          }
        </tr>
      }
    </tbody>
  </table>

// Simple user component
let <UserDisplay user:User /> =
  <div>
    <img src={user.avatarUrl}/>
    <h3>{user.name}</h3>
    <span>{user.email}</span>
  </div>

let <UserCard user:User className:string content:Element[]/> =
  <section value=<MyValue value={user.name}/> class="card {className}">
    if isLoading {
      <Spinner/>
      if error {
        <ErrorPanel/>
      }
    } else {
      <div>
        {content}
      </div>
    }
  </section>

  <abc>
    if isLoading {
        <Section>
        </Section>

        <SelfClosing/>
    } else {


    }

    if foo is {
      foo => {
        if bar is {
          bar => <BarCase/>
          else => <FallbackCase/>
        }
      }
      else => <OtherFooCase/>
    }
  </abc>

<UserCard
  if isLoading: user={user} /if

  if foo is {
    1 => className="primary"
    else => className="secondary"
  }
  className="primary">

  <:uitext>Hello, {user.name}!</>
</UserCard>




```tsx
import {
	UserMessage,
	AssistantMessage,
	PromptElement,
	BasePromptElementProps,
	PrioritizedList,
} from '@vscode/prompt-tsx';
import { ChatContext, ChatRequestTurn, ChatResponseTurn, ChatResponseMarkdownPart } from 'vscode';

interface IHistoryMessagesProps extends BasePromptElementProps {
	history: ChatContext['history'];
}

export class HistoryMessages extends PromptElement<IHistoryMessagesProps> {
	render(): PromptPiece {
		const history: (UserMessage | AssistantMessage)[] = [];
		for (const turn of this.props.history) {
			if (turn instanceof ChatRequestTurn) {
				history.push(<UserMessage>{turn.prompt}</UserMessage>);
			} else if (turn instanceof ChatResponseTurn) {
				history.push(
					<AssistantMessage name={turn.participant}>
						{chatResponseToMarkdown(turn)}
					</AssistantMessage>
				);
			}
		}
		return (
			<PrioritizedList priority={0} descending={false}>
				{history}
			</PrioritizedList>
		);
	}
}
```

```nx
  <PrioritizedList priority={0} descending={false}>
    for turn in this.props.history {
      if turn is ChatRequestTurn {
        <UserMessage>{turn.prompt}</UserMessage>
      }
      else if turn is ChatResponseTurn {
          <AssistantMessage name={turn.participant}>
            {chatResponseToMarkdown(turn)}
          </AssistantMessage>
      }
    }
  </PrioritizedList>
```


```nx
  <PrioritizedList priority={0} descending={false}>
    for turn in this.props.history {
      if turn is {
        ChatRequestTurn:
          <UserMessage>{turn.prompt}</UserMessage>

        ChatResponseTurn:
          <AssistantMessage name={turn.participant}>
            {chatResponseToMarkdown(turn)}
          </AssistantMessage>
      }
    }
  </PrioritizedList>
```


```nx
  <PrioritizedList priority={0} descending={false}>
    for turn in this.props.history {
      if {
        turn is ChatRequestTurn:
          <UserMessage>{turn.prompt}</UserMessage>

        turn is ChatResponseTurn:
          <AssistantMessage name={turn.participant}>
            {chatResponseToMarkdown(turn)}
          </AssistantMessage>
      }
    }
  </PrioritizedList>
```
